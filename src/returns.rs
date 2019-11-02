//! All the kinds of structures that can be returned from different
//! resource functons

use crate::Inode;

use std::{
    borrow::Cow,
    ffi::OsStr,
};

use fuse::{FileAttr, FileType};
use time::Timespec;

/// Like `fuse::ReplyAttr`
#[derive(Debug, Clone, Copy)]
pub struct Attr {
    /// FIXME: Document me
    pub ttl: Timespec,
    /// The inner fuse file attributes
    pub inner: FileAttr,
}

impl<T> From<T> for Attr
where
    T: Into<FileAttr>
{
    fn from(attr: T) -> Self {
        Self {
            ttl: Timespec::new(0, 0),
            inner: attr.into(),
        }
    }
}

/// Like `fuse::ReplyEntry`
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// The inner attributes
    pub attr: Attr,
    /// FIXME: Document me
    pub generation: u64,
}

impl<T> From<T> for Entry
where
    T: Into<Attr>
{
    fn from(attr: T) -> Self {
        Self {
            attr: attr.into(),
            generation: 0,
        }
    }
}

/// Like `fuse::ReplyDirectory`
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// The inode associated with the file
    pub inode: Inode,
    /// The type of the file
    pub filetype: FileType,
    /// The name of the file
    pub name: Cow<'static, OsStr>,
}
impl DirEntry {
    /// Create a new instance
    pub fn new<S>(inode: Inode, filetype: FileType, name: S) -> Self
    where
        S: Into<Cow<'static, OsStr>>
    {
        Self { inode, filetype, name: name.into() }
    }
}
