# Popcorn
An Adobe Flash clone made in Rust.

## macOS application

`cargo run` uses `popcorn.png` as its Dock icon, embedded in the executable.

To create a local app with the same icon in Finder and the Dock:

```sh
bash scripts/bundle-macos.sh debug
open target/bundle/Popcorn.app
```

Omit `debug` to build in release mode. The bundle is locally ad-hoc signed, not
notarized for distribution. Its `.icns` icon is generated from `popcorn.png`.
