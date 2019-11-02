# easyfuse

An ambigious attempt to wrap the amazing
[rust-fuse](https://github.com/zargony/rust-fuse) project in idiomatic
Rust.

The regular fuse project requires you to write code like

```rust
fn operation(&mut self, req: &Request, inode: u64, reply: ReplyKind) {
    let value = match thing() {
        Ok(ok) => ok,
        Err(err) => {
            reply.error(err);
            return;
        },
    };
    reply.do_thing(value + 1);
}
```

, mainly because instead of using return values like `Result`, they
use different `Reply` structs as arguments. On top of it, most/all
`Reply` structs have an `error` function, but there's no common
`trait` that adds these.

So while the fuse project is great for low-level applications, you can
see how writing bigger projects in it can take a lot of time and
boilerplate. With a tiny bit of abstraction, we can do better.

```rust
fn operation(&mut self, req: &mut Request) -> Result<returns::Kind> {
    let value = thing()?;
    value + 1
}
```

---

Meet EasyFuse. It's a high-level wrapper around rust-fuse that

1. Lets each trait function return a `Result`.
1. Defines newtypes for different integer types to avoid mixing them
   up.
1. Makes each node into its own struct, making it easy to combine
   different filesystem components into one larger filesystem, like
   the holy GNU intended.

EasyFuse changes the way you implement fuse filesystems. Instead of
implementing an entire filesystem, you simply implement one file or
one directory (commonly called one "resource"), and then compose them
together.

It also comes with a few standard resource types, such as a static
directory. This is useful for when you want to glue together multiple
dynamic filesystems with a static prefix.
