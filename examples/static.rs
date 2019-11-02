use easyfuse::{dir, file, returns, AttrBuilder, EasyFuse};

fn main() -> std::io::Result<()> {
    env_logger::init();

    let mut fuse = EasyFuse::new();

    let mut root = dir::StaticDirectory::new(returns::Attr::from(
        AttrBuilder::directory().build()
    ));

    root.bind(
        "README.md",
        fuse.register({
            let mut file = file::StaticFile::new(returns::Attr::from(
                AttrBuilder::file().build()
            ));
            file.set_content("# I'm a fake file\n\n\
                              Can you believe it? I don't really exist... :O\n");
            file
        })
    );
    root.bind(
        "secret",
        fuse.register({
            let mut file = file::StaticFile::new(returns::Attr::from(
                AttrBuilder::file()
                    .with_perm(0o000)
                    .build()
            ));
            file.set_content("The meaning of life is 42.\n");
            file
        })
    );

    fuse.set_root(root);

    fuse::mount(fuse, &"test-mount", &[])?;
    Ok(())
}
