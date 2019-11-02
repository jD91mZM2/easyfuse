//! easyfuse: A convenient library that lets you build your own FUSE
//! filesystems quickly without having to resolve all inodes
//! yourself. It's meant to allow flexibility but the most important
//! goal is making it easy to write idiomatic rust (an example of this
//! is using the `Result` return type instead of calling a return
//! function).

#![warn(
    // Harden built-in lints
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,

    // Harden clippy lints
    clippy::cargo_common_metadata,
    clippy::clone_on_ref_ptr,
    clippy::dbg_macro,
    clippy::decimal_literal_representation,
    clippy::float_cmp_const,
    clippy::get_unwrap,
    clippy::integer_arithmetic,
    clippy::integer_division,
    clippy::pedantic,
    clippy::print_stdout,
)]

use std::{
    collections::BTreeMap,
    convert::TryInto,
    ffi::OsStr,
    fmt,
    path::Path,
};

use fuse::{
    // ReplyBmap,
    // ReplyCreate,
    // ReplyLock,
    // ReplyStatfs,
    // ReplyWrite,
    // ReplyXattr,
    Filesystem,
    FileType,
    ReplyAttr,
    ReplyData,
    ReplyDirectory,
    ReplyEmpty,
    ReplyEntry,
    ReplyOpen,
    Request as FuseRequest,
};
use log::trace;

pub mod cell;
pub mod resource;
pub mod returns;

pub use cell::*;
pub use resource::*;

/// A result type that defaults to using `c_int` as error
pub type Result<T, E = libc::c_int> = std::result::Result<T, E>;

const ROOT_ID: Inode = Inode(1);

/// A `Filesystem` implementation that resolves inodes automatically
/// and uses return values in a more idiomatic way
pub struct EasyFuse {
    nodes: BTreeMap<Inode, ResourceCell>,
    next_inode: Inode,
}
impl Default for EasyFuse {
    fn default() -> Self {
        #[allow(clippy::integer_arithmetic)]
        Self {
            nodes: BTreeMap::new(),
            next_inode: Inode(ROOT_ID.0 + 1),
        }
    }
}
impl fmt::Debug for EasyFuse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EasyFuse")
    }
}
impl EasyFuse {
    /// Same as `Self::default()`
    pub fn new() -> Self {
        Self::default()
    }
    /// Same as `try_register`, but panics on the unlikely case of
    /// integer overflow
    pub fn register<R>(&mut self, resource: R) -> Inode
    where
        R: Into<ResourceCell>
    {
        self.try_register(resource).expect("integer overflow")
    }
    /// Bind an inode to a resource. Note that this won't make the
    /// resource be indexed anywhere and so only access with the exact
    /// inode specified will be affected if you only run this.
    pub fn try_register<R>(&mut self, resource: R) -> Option<Inode>
    where
        R: Into<ResourceCell>
    {
        let id = self.next_inode;
        self.next_inode = Inode(id.0.checked_add(1)?);

        self.nodes.insert(id, resource.into());
        Some(id)
    }
    /// Remove a binding from a certain inode, and return the previous
    /// associated resource, if any
    pub fn unregister(&mut self, inode: Inode) -> Option<ResourceCell> {
        self.nodes.remove(&inode)
    }

    /// Resolve an inode to a resource
    pub fn resolve(&mut self, inode: Inode) -> Option<ResourceCell> {
        self.nodes.get(&inode).cloned()
    }

    /// Bind a resource to a hardcoded root inode ID
    pub fn set_root<R>(&mut self, resource: R) -> Option<ResourceCell>
    where
        R: Into<ResourceCell>
    {
        self.nodes.insert(ROOT_ID, resource.into())
    }

    fn request<'a>(&'a mut self, inode: Inode, req: &'a FuseRequest) -> Request<'a> {
        Request {
            inner: req,
            fs: self,
            inode,
        }
    }
}

macro_rules! attempt {
    ($reply:expr, $result:expr) => {
        match $result {
            Ok(ok) => ok,
            Err(err) => {
                $reply.error(err);
                return;
            },
        }
    }
}

impl Filesystem for EasyFuse {
    fn getattr(&mut self, req: &FuseRequest, ino: u64, reply: ReplyAttr) {
        let ino = Inode(ino);
        let node = attempt!(reply, self.resolve(ino).ok_or(libc::ENOENT));

        let result = node.borrow_mut().getattr(&mut self.request(ino, req));
        trace!("getattr(...) = {:#?}", result);
        let mut attr = attempt!(reply, result);
        attr.inner.ino = ino.0;
        reply.attr(&attr.ttl, &attr.inner);
    }

    //  ____  _                                   _   _
    // |  _ \(_)_ __    ___  _ __   ___ _ __ __ _| |_(_) ___  _ __  ___
    // | | | | | '__|  / _ \| '_ \ / _ \ '__/ _` | __| |/ _ \| '_ \/ __|
    // | |_| | | |    | (_) | |_) |  __/ | | (_| | |_| | (_) | | | \__ \
    // |____/|_|_|     \___/| .__/ \___|_|  \__,_|\__|_|\___/|_| |_|___/
    //                      |_|

    fn lookup(&mut self, req: &FuseRequest, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let parent = Inode(parent);
        let node = attempt!(reply, self.resolve(parent).ok_or(libc::ENOENT));

        let result = node.borrow_mut().lookup(&mut self.request(parent, req), name);
        trace!("lookup(...) = {:#?}", result);
        let entry = attempt!(reply, result);
        reply.entry(&entry.attr.ttl, &entry.attr.inner, entry.generation);
    }
    fn readdir(&mut self, req: &FuseRequest, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let ino = Inode(ino);
        let node = attempt!(reply, self.resolve(ino).ok_or(libc::ENOENT));
        let mut entries = vec![
            returns::DirEntry::new(ino, FileType::Directory, OsStr::new(".")),
            returns::DirEntry::new(ino, FileType::Directory, OsStr::new("..")),
        ];

        let result = node.borrow_mut().readdir(&mut self.request(ino, req), &mut entries);
        trace!("readdir(...) = {:?}", result);
        attempt!(reply, result);

        let mut i = 1;
        for entry in entries.into_iter().skip(offset.try_into().unwrap_or(0)) {
            reply.add(entry.inode.0, i, entry.filetype, &entry.name);
            i = i.checked_add(1).expect("integer overflow");
        }
        reply.ok();
    }
    fn symlink(&mut self, req: &FuseRequest, parent: u64, name: &OsStr, link: &Path, reply: ReplyEntry) {
        let parent = Inode(parent);
        let node = attempt!(reply, self.resolve(parent).ok_or(libc::ENOENT));

        let result = node.borrow_mut().symlink(&mut self.request(parent, req), name, link);
        trace!("symlink(...) = {:#?}", result);
        let mut entry = attempt!(reply, result);

        entry.attr.inner.ino = parent.0;
        reply.entry(&entry.attr.ttl, &entry.attr.inner, entry.generation);
    }

    //  _____ _ _                                   _   _
    // |  ___(_) | ___    ___  _ __   ___ _ __ __ _| |_(_) ___  _ __  ___
    // | |_  | | |/ _ \  / _ \| '_ \ / _ \ '__/ _` | __| |/ _ \| '_ \/ __|
    // |  _| | | |  __/ | (_) | |_) |  __/ | | (_| | |_| | (_) | | | \__ \
    // |_|   |_|_|\___|  \___/| .__/ \___|_|  \__,_|\__|_|\___/|_| |_|___/
    //                        |_|

    fn open(&mut self, req: &FuseRequest, ino: u64, flags: u32, reply: ReplyOpen) {
        let ino = Inode(ino);
        let node = attempt!(reply, self.resolve(ino).ok_or(libc::ENOENT));

        let result = node.borrow_mut().open(&mut self.request(ino, req), flags);
        trace!("open(...) = {:?}", result);
        let handle = attempt!(reply, result);

        reply.opened(handle.0, 0);
    }
    fn release(&mut self, req: &FuseRequest, ino: u64, fh: u64, flags: u32, _lock_owner: u64, _flush: bool, reply: ReplyEmpty) {
        let ino = Inode(ino);
        let node = attempt!(reply, self.resolve(ino).ok_or(libc::ENOENT));

        let result = node.borrow_mut().close(&mut self.request(ino, req), FileHandle(fh), flags);
        trace!("close(...) = {:?}", result);
        attempt!(reply, result);

        reply.ok();
    }
    fn read(&mut self, req: &FuseRequest, ino: u64, fh: u64, offset: i64, len: u32, reply: ReplyData) {
        let ino = Inode(ino);
        let node = attempt!(reply, self.resolve(ino).ok_or(libc::ENOENT));
        {
            let mut node = node.borrow_mut();

            let result = node.read(&mut self.request(ino, req), FileHandle(fh), offset, len);
            trace!("read(...) = {:?}", result);
            let buf = attempt!(reply, result);

            assert!(
                buf.len() <= len.try_into().unwrap_or(usize::max_value()),
                "Number of read bytes should never exceed numbers of requested bytes"
            );
            reply.data(&buf);
        }
    }

    //  _____ ___  ____   ___
    // |_   _/ _ \|  _ \ / _ \
    //   | || | | | | | | | | |
    //   | || |_| | |_| | |_| |
    //   |_| \___/|____/ \___/

    /*
    // ENOSYS
    fn setattr(&mut self, _req: &FuseRequest, _ino: u64, _mode: Option<u32>, _uid: Option<u32>, _gid: Option<u32>, _size: Option<u64>, _atime: Option<Timespec>, _mtime: Option<Timespec>, _fh: Option<u64>, _crtime: Option<Timespec>, _chgtime: Option<Timespec>, _bkuptime: Option<Timespec>, _flags: Option<u32>, reply: ReplyAttr) {
        reply.error(libc::ENOSYS);
    }
    fn readlink(&mut self, _req: &FuseRequest, _ino: u64, reply: ReplyData) {
        reply.error(libc::ENOSYS);
    }
    fn mknod(&mut self, _req: &FuseRequest, _parent: u64, _name: &OsStr, _mode: u32, _rdev: u32, reply: ReplyEntry) {
        reply.error(libc::ENOSYS);
    }
    fn mkdir(&mut self, _req: &FuseRequest, _parent: u64, _name: &OsStr, _mode: u32, reply: ReplyEntry) {
        reply.error(libc::ENOSYS);
    }
    fn unlink(&mut self, _req: &FuseRequest, _parent: u64, _name: &OsStr, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn rmdir(&mut self, _req: &FuseRequest, _parent: u64, _name: &OsStr, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn rename(&mut self, _req: &FuseRequest, _parent: u64, _name: &OsStr, _newparent: u64, _newname: &OsStr, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn link(&mut self, _req: &FuseRequest, _ino: u64, _newparent: u64, _newname: &OsStr, reply: ReplyEntry) {
        reply.error(libc::ENOSYS);
    }
    fn write(&mut self, _req: &FuseRequest, _ino: u64, _fh: u64, _offset: i64, _data: &[u8], _flags: u32, reply: ReplyWrite) {
        reply.error(libc::ENOSYS);
    }
    fn flush(&mut self, _req: &FuseRequest, _ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn fsync(&mut self, _req: &FuseRequest, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn fsyncdir(&mut self, _req: &FuseRequest, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn setxattr(&mut self, _req: &FuseRequest, _ino: u64, _name: &OsStr, _value: &[u8], _flags: u32, _position: u32, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn getxattr(&mut self, _req: &FuseRequest, _ino: u64, _name: &OsStr, _size: u32, reply: ReplyXattr) {
        reply.error(libc::ENOSYS);
    }
    fn listxattr(&mut self, _req: &FuseRequest, _ino: u64, _size: u32, reply: ReplyXattr) {
        reply.error(libc::ENOSYS);
    }
    fn removexattr(&mut self, _req: &FuseRequest, _ino: u64, _name: &OsStr, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn access(&mut self, _req: &FuseRequest, _ino: u64, _mask: u32, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn create(&mut self, _req: &FuseRequest, _parent: u64, _name: &OsStr, _mode: u32, _flags: u32, reply: ReplyCreate) {
        reply.error(libc::ENOSYS);
    }
    fn getlk(&mut self, _req: &FuseRequest, _ino: u64, _fh: u64, _lock_owner: u64, _start: u64, _end: u64, _typ: u32, _pid: u32, reply: ReplyLock) {
        reply.error(libc::ENOSYS);
    }
    fn setlk(&mut self, _req: &FuseRequest, _ino: u64, _fh: u64, _lock_owner: u64, _start: u64, _end: u64, _typ: u32, _pid: u32, _sleep: bool, reply: ReplyEmpty) {
        reply.error(libc::ENOSYS);
    }
    fn bmap(&mut self, _req: &FuseRequest, _ino: u64, _blocksize: u32, _idx: u64, reply: ReplyBmap) {
        reply.error(libc::ENOSYS);
    }

    // Has default impls
    fn init(&mut self, _req: &FuseRequest) -> Result<(), libc::c_int> {
        Ok(())
    }
    fn destroy(&mut self, _req: &FuseRequest) {}
    fn forget(&mut self, _req: &FuseRequest, _ino: u64, _nlookup: u64) {}
    fn opendir(&mut self, _req: &FuseRequest, _ino: u64, _flags: u32, reply: ReplyOpen) {
        reply.opened(0, 0);
    }
    fn releasedir(&mut self, _req: &FuseRequest, _ino: u64, _fh: u64, _flags: u32, reply: ReplyEmpty) {
        reply.ok();
    }
    fn statfs(&mut self, _req: &FuseRequest, _ino: u64, reply: ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }
     */
}
