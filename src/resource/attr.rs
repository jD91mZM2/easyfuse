//! Convenient builder for `FileAttr`s

use fuse::{FileAttr, FileType};
use time::Timespec;

macro_rules! attr_builder {
    ($($property:ident $setter:ident: $type:ty = |$self:ident| $default:expr,)*) => {
        /// A builder of `FileAttr` with sane default values
        #[derive(Debug, Default, Clone, Copy)]
        pub struct AttrBuilder {
            $($property: Option<$type>,)*
        }
        impl AttrBuilder {
            /// Same as `AttrBuilder::default()`
            pub fn file() -> Self {
                Self::default()
            }
            /// Convenience for
            /// ```rust,ignore
            /// AttrBuilder::default()
            ///     .with_kind(FileType::Directory)
            /// ```
            pub fn directory() -> Self {
                Self::default()
                    .with_kind(FileType::Directory)
            }

            /// Build a `FileAttr`, resolving all default values
            pub fn build(self) -> FileAttr {
                FileAttr {
                    $($property: self.$property.unwrap_or_else(|| {
                        let $self = &self;
                        $default
                    }),)*
                }
            }

            $(
                /// A chaining function to set the value of a
                /// property. Generated in bulk by a macro.
                pub fn $setter<T>(mut self, $property: T) -> Self
                where
                    T: Into<Option<$type>>,
                {
                    self.$property = $property.into();
                    self
                }
            )*
        }
    }
}

attr_builder! {
    ino     with_ino:     u64       = |_attrs|  0,
    size    with_size:    u64       = |_attrs|  0,
    blocks  with_blocks:  u64       = |_attrs|  0,
    atime   with_atime:   Timespec  = |attrs|   attrs.mtime.unwrap_or_else(|| time::now().to_timespec()),
    mtime   with_mtime:   Timespec  = |attrs|   attrs.ctime.unwrap_or_else(|| time::now().to_timespec()),
    ctime   with_ctime:   Timespec  = |_attrs|  time::now().to_timespec(),
    crtime  with_crtime:  Timespec  = |attrs|   attrs.ctime.unwrap_or_else(|| time::now().to_timespec()),
    kind    with_kind:    FileType  = |_attrs|  FileType::RegularFile,
    perm    with_perm:    u16       = |attrs|   if attrs.kind == Some(FileType::Directory) { 0o555 } else { 0o444 },
    nlink   with_nlink:   u32       = |_attrs|  0,
    uid     with_uid:     u32       = |_attrs|  unsafe { libc::getuid() },
    gid     with_gid:     u32       = |_attrs|  unsafe { libc::getgid() },
    rdev    with_rdev:    u32       = |_attrs|  0,
    flags   with_flags:   u32       = |_attrs|  0,
}
