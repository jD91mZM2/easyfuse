//! A resource means either a file or directory, or any other type of
//! filesystem node. `EasyFuse` is built upon different generic
//! resources being combined together to form a filesystem.

use crate::{returns, EasyFuse, Result};

use std::{
    borrow::Cow,
    convert::TryFrom,
    ffi::OsStr,
    path::Path,
};

use fuse::FileAttr;

pub mod attr;
pub mod newtypes;
pub mod dir;
pub mod file;

pub use attr::*;
pub use newtypes::*;

/// Data common for all request types
#[derive(Debug)]
pub struct Request<'a> {
    /// The inner FUSE request parameters
    pub inner: &'a fuse::Request<'a>,
    /// The core file system, which has the possibility to lookup
    /// resources by inodes or register new resources.
    pub fs: &'a mut EasyFuse,
    /// The inode of the current resource.
    pub inode: Inode,
}
impl<'a> Request<'a> {
    /// Return the relevant permission digit when the current user
    /// tries to open a specific file.
    #[allow(clippy::integer_arithmetic)] // clippy is dumb
    pub fn perms(&self, attrs: &FileAttr) -> Permissions {
        let perms = if self.inner.uid() == attrs.uid {
            (attrs.perm & 0o700) >> (3*2)
        } else if self.inner.gid() == attrs.gid {
            (attrs.perm & 0o070) >> 3
        } else {
            (attrs.perm & 0o007)
        };
        u8::try_from(perms).ok()
            .and_then(Permissions::from_bits)
            .expect("Permission was not shifted correctly")
    }

    /// Compare the user permissions using `perms` and raise an
    /// `EPERM` if it's lacking.
    pub fn ensure_access(&self, attrs: &FileAttr, required: Permissions) -> Result<()> {
        if self.perms(attrs).contains(required) {
            Ok(())
        } else {
            Err(libc::EPERM)
        }
    }
}

/// A generic resource, either for a file or directory. An inode can
/// be linked to a resource to make all filesystem operations on that
/// inode get passed to here.
pub trait Resource {
    /// Get meta information of this file, for example when the `stat`
    /// system call is made. The `ino` value returned here will be
    /// overwritten with this resource's inode, so set it to zero.
    fn getattr(&mut self, _req: &mut Request) -> Result<returns::Attr> {
        Err(libc::ENOSYS)
    }

    //  ____  _                                   _   _
    // |  _ \(_)_ __    ___  _ __   ___ _ __ __ _| |_(_) ___  _ __  ___
    // | | | | | '__|  / _ \| '_ \ / _ \ '__/ _` | __| |/ _ \| '_ \/ __|
    // | |_| | | |    | (_) | |_) |  __/ | | (_| | |_| | (_) | | | \__ \
    // |____/|_|_|     \___/| .__/ \___|_|  \__,_|\__|_|\___/|_| |_|___/
    //                      |_|

    /// Convert a path to a child of this resource to an inode,
    /// assuming it's a directory
    fn lookup(&mut self, _req: &mut Request, _path: &OsStr) -> Result<returns::Entry> {
        Err(libc::ENOSYS)
    }
    /// Read all entries of this resource, assuming it's a directory
    fn readdir(&mut self, _req: &mut Request, _output: &mut Vec<returns::DirEntry>) -> Result<()> {
        Err(libc::ENOSYS)
    }
    /// Symlink a file into this resource, assuming it's a
    /// directory. Should return the stat for the created symlink,
    /// similar to `lookup`.
    fn symlink(&'_ mut self, _req: &mut Request, _path: &OsStr, _link: &Path) -> Result<returns::Entry> {
        Err(libc::ENOSYS)
    }

    //  _____ _ _                                   _   _
    // |  ___(_) | ___    ___  _ __   ___ _ __ __ _| |_(_) ___  _ __  ___
    // | |_  | | |/ _ \  / _ \| '_ \ / _ \ '__/ _` | __| |/ _ \| '_ \/ __|
    // |  _| | | |  __/ | (_) | |_) |  __/ | | (_| | |_| | (_) | | | \__ \
    // |_|   |_|_|\___|  \___/| .__/ \___|_|  \__,_|\__|_|\___/|_| |_|___/
    //                        |_|

    /// Open a new instance of this resource, assuming it's a
    /// file. May return a "file handle", which is basically a number
    /// that doesn't mean anything to anyone but this resource
    /// itself. Generally though, it's a good idea to use the file
    /// handle to keep track of which instance is which, such as
    /// through a raw pointer or an ID.
    fn open(&mut self, _req: &mut Request, _flags: u32) -> Result<FileHandle> {
        Ok(FileHandle(0))
    }

    /// Close an instance of this resource, assuming it's a
    /// file. Files are reference counted, but this will only be
    /// called once the final copy of a file is closed. Any errors are
    /// ignored by FUSE.
    fn close(&mut self, _req: &mut Request, _fh: FileHandle, _flags: u32) -> Result<()> {
        Ok(())
    }

    /// Read contents of this resource from a specific offset into a
    /// buffer, assuming it's a file. Should return the number of
    /// bytes read, which must never be more than `buf.len()`.
    fn read(&'_ mut self, _req: &mut Request, _fh: FileHandle, _offset: i64, _len: u32) -> Result<Cow<'_, [u8]>> {
        Err(libc::ENOSYS)
    }
}

/// Abstraction on top of resource that errors on any attempt to use a
/// directory operation. Anything that implements `File` can be used
/// as a resource using the `FileResource` wrapper.
pub trait File {
    /// See `Resource::getattr`. Doesn't have a default ENOSYS
    /// implementation because most GNU tools fail if this isn't
    /// implemented.
    fn getattr(&mut self, _req: &mut Request) -> Result<returns::Attr>;
    /// See `Resource::open`
    fn open(&mut self, _req: &mut Request, _flags: u32) -> Result<FileHandle> {
        Ok(FileHandle(0))
    }
    /// See `Resource::close`
    fn close(&mut self, _req: &mut Request, _fh: FileHandle, _flags: u32) -> Result<()> {
        Ok(())
    }
    /// See `Resource::read`
    fn read(&'_ mut self, _req: &mut Request, _fh: FileHandle, _offset: i64, _len: u32) -> Result<Cow<'_, [u8]>> {
        Err(libc::ENOSYS)
    }
}

/// See the `File` trait. Because a type can technically implement
/// both `File` and `Directory`, Rust forces us (for good reasons!)
/// to have a wrapper here so the user can choose.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct FileResource<F: File>(pub F);

impl<F: File> From<F> for FileResource<F> {
    fn from(file: F) -> Self {
        Self(file)
    }
}

impl<F: File> Resource for FileResource<F> {
    fn getattr(&mut self, req: &mut Request) -> Result<returns::Attr> {
        self.0.getattr(req)
    }

    // Directory operations

    fn lookup(&mut self, _req: &mut Request, _path: &OsStr) -> Result<returns::Entry> {
        Err(libc::EBADF)
    }
    fn readdir(&mut self, _req: &mut Request, _output: &mut Vec<returns::DirEntry>) -> Result<()> {
        Err(libc::EBADF)
    }
    fn symlink(&'_ mut self, _req: &mut Request, _path: &OsStr, _link: &Path) -> Result<returns::Entry> {
        Err(libc::EBADF)
    }

    // File operations

    fn open(&mut self, req: &mut Request, flags: u32) -> Result<FileHandle> {
        self.0.open(req, flags)
    }
    fn close(&mut self, req: &mut Request, fh: FileHandle, flags: u32) -> Result<()> {
        self.0.close(req, fh, flags)
    }
    fn read(&'_ mut self, req: &mut Request, fh: FileHandle, offset: i64, len: u32) -> Result<Cow<'_, [u8]>> {
        self.0.read(req, fh, offset, len)
    }
}

/// Abstraction on top of resource that errors on any attempt to use a
/// directory operation. Anything that implements `Directory` will
/// automatically implement `Resource`.
pub trait Directory {
    /// See `Resource::getattr`. Doesn't have a default ENOSYS
    /// implementation because most GNU tools fail if this isn't
    /// implemented.
    fn getattr(&mut self, _req: &mut Request) -> Result<returns::Attr>;
    /// See `Resource::lookup`
    fn lookup(&mut self, _req: &mut Request, _path: &OsStr) -> Result<returns::Entry> {
        Err(libc::ENOSYS)
    }
    /// See `Resource::readdir`
    fn readdir(&mut self, _req: &mut Request, _output: &mut Vec<returns::DirEntry>) -> Result<()> {
        Err(libc::ENOSYS)
    }
    /// See `Resource::symlink`
    fn symlink(&'_ mut self, _req: &mut Request, _path: &OsStr, _link: &Path) -> Result<returns::Entry> {
        Err(libc::ENOSYS)
    }
}

/// See the `Directory` trait. Because a type can technically implement
/// both `File` and `Directory`, Rust forces us (for good reasons!)
/// to have a wrapper here so the user can choose.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct DirectoryResource<D: Directory>(pub D);

impl<D: Directory> From<D> for DirectoryResource<D> {
    fn from(directory: D) -> Self {
        Self(directory)
    }
}

impl<D: Directory> Resource for DirectoryResource<D> {
    fn getattr(&mut self, req: &mut Request) -> Result<returns::Attr> {
        self.0.getattr(req)
    }

    // Directory operations

    fn lookup(&mut self, req: &mut Request, path: &OsStr) -> Result<returns::Entry> {
        self.0.lookup(req, path)
    }
    fn readdir(&mut self, req: &mut Request, output: &mut Vec<returns::DirEntry>) -> Result<()> {
        self.0.readdir(req, output)
    }
    fn symlink(&'_ mut self, req: &mut Request, path: &OsStr, link: &Path) -> Result<returns::Entry> {
        self.0.symlink(req, path, link)
    }

    // File operations

    fn open(&mut self, _req: &mut Request, _flags: u32) -> Result<FileHandle> {
        Err(libc::EBADF)
    }
    fn close(&mut self, _req: &mut Request, _fh: FileHandle, _flags: u32) -> Result<()> {
        Err(libc::EBADF)
    }
    fn read(&'_ mut self, _req: &mut Request, _fh: FileHandle, _offset: i64, _len: u32) -> Result<Cow<'_, [u8]>> {
        Err(libc::EBADF)
    }
}
