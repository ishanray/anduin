# Anduin

A fast Git GUI for coding-agent workflows and worktrees, built with Rust and [Iced](https://iced.rs).

## Build

Requires Rust 1.85+.

```
cargo build --release
```

The binary is at `target/release/Anduin`.

### macOS app bundle

```
cargo install cargo-bundle
cargo bundle --release
```

### Deploy locally (macOS)

```
./scripts/deploy.sh
```

This builds a release bundle, copies it to `/Applications/Anduin.app`, and relaunches.

## License

[Unlicense](LICENSE)
