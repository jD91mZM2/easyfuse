//! Different `Resource` implementations for directory-like nodes

use crate::{
    returns,
    Directory,
    Inode,
    Request,
    Result,
};

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
};

use fuse::FileType;

/// A simple directory that you can register files on to
#[derive(Debug)]
pub struct StaticDirectory {
    binds: HashMap<OsString, Inode>,
    attr: returns::Attr,
}
impl StaticDirectory {
    /// Create a new instance from a file attribute
    pub fn new(attr: returns::Attr) -> Self {
        Self {
            binds: HashMap::new(),
            attr,
        }
    }

    /// Getter for the inner file attributes
    pub fn attr(&self) -> &returns::Attr {
        &self.attr
    }
    /// Setter for the inner file attributes
    pub fn set_attr<T>(&mut self, attr: T)
    where
        T: Into<returns::Attr>
    {
        self.attr = attr.into();
    }

    /// Bind a file onto this directory
    pub fn bind<P>(&mut self, path: P, resource: Inode)
    where
        P: Into<OsString>
    {
        self.binds.insert(path.into(), resource);
    }
    /// Unbind a file from this directory
    pub fn unbind<P>(&mut self, path: P) -> Option<Inode>
    where
        P: AsRef<OsStr>
    {
        self.binds.remove(path.as_ref())
    }
}

impl Directory for StaticDirectory {
    fn getattr(&mut self, _req: &mut Request) -> Result<returns::Attr> {
        // Save the user from himself
        self.attr.inner.kind = FileType::Directory;
        Ok(self.attr)
    }
    fn lookup(&mut self, req: &mut Request, path: &OsStr) -> Result<returns::Entry> {
        let inode = *self.binds.get(path).ok_or(libc::ENOENT)?;
        let resource = req.fs.resolve(inode).expect("invalid inode bound to StaticDirectory");
        let mut stat = resource.borrow_mut().getattr(req)?;
        stat.inner.ino = inode.0;
        Ok(returns::Entry::from(stat))
    }
    fn readdir(&mut self, req: &mut Request, output: &mut Vec<returns::DirEntry>) -> Result<()> {
        for (path, &inode) in &self.binds {
            let resource = req.fs.resolve(inode).expect("invalid inode bound to StaticDirectory");
            let mut stat = resource.borrow_mut().getattr(req)?;
            stat.inner.ino = inode.0;
            output.push(returns::DirEntry::new(inode, stat.inner.kind, path.clone()))
        }
        Ok(())
    }
}
