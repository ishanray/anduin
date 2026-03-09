# Packaging Anduin

Anduin uses `cargo-bundle` metadata in `Cargo.toml` for native app packaging.

## Install

```bash
cargo install cargo-bundle
```

## Build bundles

### macOS

```bash
cargo bundle --release --bin Anduin --format osx
```

Produces a `.app` bundle under `target/release/bundle/osx/`.

### Linux

```bash
cargo bundle --release --bin Anduin --format deb
```

Produces a `.deb` package under `target/release/bundle/deb/`.

A sample desktop entry is also included at `packaging/linux/anduin.desktop`.

### Windows

```powershell
cargo bundle --release --bin Anduin --target x86_64-pc-windows-msvc --format msi
```

Produces an `.msi` installer under the target bundle directory.

## Notes

- App metadata lives in `[package.metadata.bundle]` in `Cargo.toml`.
- macOS extra `Info.plist` keys live in `packaging/macos/Info.plist.ext`.
- Release builds on Windows use the GUI subsystem, so no console window appears.
- Linux keeps terminal detaching at runtime when launched directly from a shell.
- macOS and Windows run as normal foreground GUI apps.
