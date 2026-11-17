# Packaging

This directory holds the files for the first two distribution channels:

1. **GitHub Releases** — prebuilt binaries, checksums, and a tag-triggered
   workflow.
2. **crates.io, AUR, and a Homebrew tap** — source installs.

The workflow in `.github/workflows/release.yml` builds and uploads the GitHub
Release assets. The AUR and Homebrew files for `v0.1.0` already carry the
source tarball checksum; for later releases, refresh them with the checksum
from the new tag.

Live locations:

- GitHub release: <https://github.com/oak443/complete-album/releases/tag/v0.1.0>
- Homebrew tap: <https://github.com/oak443/homebrew-complete-album>
- AUR: prepared in `packaging/aur/`; upload it to the AUR when you have an AUR
  account.

## 0. Before the first release

Replace `oak443` in these files if your repository lives elsewhere:

- `Cargo.toml` (`repository`)
- `packaging/aur/PKGBUILD` (`url`, `source`)
- `packaging/aur/.SRCINFO`
- `packaging/homebrew/Formula/complete-album.rb` (`homepage`, `url`, `head`)

Set the AUR maintainer line in `packaging/aur/PKGBUILD` to a real name and
email. Do not upload a package with a placeholder address.

## 1. GitHub Release

```bash
git tag v0.1.0
git push origin v0.1.0
```

The tag starts `.github/workflows/release.yml`, which:

1. runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`;
2. builds:
   - `x86_64-unknown-linux-musl`
   - `aarch64-unknown-linux-musl`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-pc-windows-msvc`
3. packages each target as `.tar.gz` or `.zip` with `README.md`, `LICENSE`,
   and `CHANGELOG.md`;
4. writes `SHA256SUMS`;
5. creates the GitHub Release.

Linux uses musl static binaries so the downloaded executable does not depend
on the target machine's glibc. The two Linux targets are built with `cross`.

## 2. crates.io

The crate name is currently free. Check again before publishing:

```bash
cargo search complete-album
```

Then:

```bash
cargo login
cargo publish --dry-run
cargo publish
```

After publishing, users can install with:

```bash
cargo install complete-album
```

## 3. AUR

The source package is `packaging/aur/PKGBUILD`. After the GitHub release:

```bash
VERSION=0.1.0
curl -L "https://github.com/oak443/complete-album/archive/refs/tags/v${VERSION}.tar.gz" \
  | sha256sum
```

Put that checksum into `sha256sums` in `PKGBUILD`, then regenerate the metadata:

```bash
cd packaging/aur
makepkg --printsrcinfo > .SRCINFO
makepkg -si
```

Upload `PKGBUILD` and `.SRCINFO` to the AUR as `complete-album`.

For faster installs, an AUR `complete-album-bin` package can be added later
using the `x86_64-unknown-linux-musl` Release asset.

## 4. Homebrew

Create a tap repository, for example `homebrew-complete-album`, and copy the
formula to `Formula/complete-album.rb`.

After the GitHub release:

```bash
curl -L "https://github.com/oak443/complete-album/archive/refs/tags/v0.1.0.tar.gz" \
  | shasum -a 256
```

Put that checksum into the formula's `sha256`, then test locally:

```bash
brew install --build-from-source ./Formula/complete-album.rb
brew test complete-album
brew audit --strict --online complete-album
```

Users then install with:

```bash
brew install oak443/complete-album/complete-album
```

## Release checklist

- [ ] `Cargo.toml` version matches the tag (`v0.1.0` → `0.1.0`)
- [ ] `CHANGELOG.md` has the version's section
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `cargo package --allow-dirty`
- [ ] `git tag v0.1.0 && git push origin v0.1.0`
- [ ] GitHub Release assets and `SHA256SUMS` look right
- [ ] `cargo publish`
- [ ] AUR checksum updated and `makepkg -si` tested
- [ ] Homebrew checksum updated and `brew test` passed
