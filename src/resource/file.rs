//! Different `Resource` implementations for file-like nodes

use crate::{
    returns,
    File,
    FileHandle,
    Permissions,
    Request,
    Result,
};

use std::{
    borrow::Cow,
    cmp,
    convert::TryInto,
};

use fuse::FileType;

/// A simple static file
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct StaticFile {
    content: Vec<u8>,
    attr: returns::Attr,
}
impl StaticFile {
    /// Create a new instance from a file attribute
    pub fn new(attr: returns::Attr) -> Self {
        Self {
            content: Vec::default(),
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

    /// Set the static data to be read from the file
    pub fn set_content<C>(&mut self, content: C)
    where
        C: Into<Vec<u8>>
    {
        self.content = content.into();
    }
    /// Getter for the inner static data to be read from the file
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}
impl File for StaticFile {
    #[allow(clippy::integer_arithmetic)] // not dividing by zero, clippy ya dumb fuck
    #[allow(clippy::integer_division)]   // i am very much aware of that this will truncate
    fn getattr(&mut self, _req: &mut Request) -> Result<returns::Attr> {
        // Save the user from himself
        self.attr.inner.kind = FileType::RegularFile;
        self.attr.inner.size = self.content.len().try_into().unwrap_or(u64::max_value());
        self.attr.inner.blocks = self.attr.inner.size / 4096;
        Ok(self.attr)
    }

    fn read(&'_ mut self, req: &mut Request, _fh: FileHandle, offset: i64, len: u32) -> Result<Cow<'_, [u8]>> {
        req.ensure_access(&self.attr.inner, Permissions::READ)?;
        let start: usize = offset.try_into().unwrap_or(0);
        let end: usize = cmp::min(
            len.try_into().ok().and_then(|len| start.checked_add(len)).expect("integer overflow"),
            self.content.len()
        );

        let buf = &self.content.get(start..end).ok_or(libc::ERANGE)?;
        Ok(Cow::Borrowed(&buf))
    }
}
