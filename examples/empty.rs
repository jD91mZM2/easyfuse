use easyfuse::{returns, Directory, DirectoryResource, EasyFuse, Request, Result};
use fuse::FileType;

struct Root;

impl Directory for Root {
    fn getattr(&mut self, _req: &mut Request) -> Result<returns::Attr> {
        Ok(returns::Attr::from(fuse::FileAttr {
            ino: 0,
            size: 0,
            blocks: 0,
            atime: time::now().to_timespec(),
            mtime: time::now().to_timespec(),
            ctime: time::now().to_timespec(),
            crtime: time::now().to_timespec(),
            kind: FileType::Directory,
            perm: 0o555,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        }))
    }
    fn readdir(&mut self, _req: &mut Request, _output: &mut Vec<returns::DirEntry>) -> Result<()> {
        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    env_logger::init();

    let mut fuse = EasyFuse::new();
    fuse.set_root(DirectoryResource(Root));

    fuse::mount(fuse, &"test-mount", &[])?;
    Ok(())
}
