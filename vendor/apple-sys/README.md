This is a stupid hack to get the subset of `apple-sys` required by `keepawake`
in building in Nix. The `apple-sys` crate [doesn't reliably generate
bindings](https://github.com/youknowone/apple-sys/issues/15), and I have yet to
find a way to make `build.rs` work reliably
