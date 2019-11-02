//! A wrapper for `Rc<RefCell<dyn Resource>>` that implements
//! `From<Resource>`

use crate::{dir, file, DirectoryResource, FileResource, Resource};
use std::{
    cell::RefCell,
    fmt,
    ops::Deref,
    rc::Rc,
};

/// Newtype for `Rc<RefCell<dyn Resource>>`
#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct ResourceCell(pub Rc<RefCell<dyn Resource>>);

impl<R> From<R> for ResourceCell
where
    R: Resource + 'static
{
    fn from(resource: R) -> Self {
        Self(Rc::new(RefCell::new(resource)))
    }
}

impl From<file::StaticFile> for ResourceCell {
    fn from(file: file::StaticFile) -> Self {
        Self::from(FileResource(file))
    }
}
impl From<dir::StaticDirectory> for ResourceCell {
    fn from(dir: dir::StaticDirectory) -> Self {
        Self::from(DirectoryResource(dir))
    }
}

impl fmt::Debug for ResourceCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceCell")
    }
}

impl Deref for ResourceCell {
    type Target = RefCell<dyn Resource>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
