//! Newtype wrappers around various integer types in order to leverage
//! the type system to avoid mixing up certain values like inodes with
//! other numeric values.

use bitflags::bitflags;

/// Newtype for an inode, see module-level docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Inode(pub u64);

/// Newtype for a file handle, see module-level docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileHandle(pub u64);

bitflags! {
    /// A single octal digit of unix permissions
    pub struct Permissions: u8 {
        /// Permission to execute this program or open this directory
        const EXECUTE = 1;
        /// Permission to write to this file
        const WRITE   = 1 << 1;
        /// Permission to read this file
        const READ    = 1 << 2;
    }
}
