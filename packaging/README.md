# Packaging

This directory holds the distribution files for:

1. **GitHub Releases** — prebuilt binaries, checksums, and a tag-triggered
   workflow.
2. **Fedora COPR** — RPM builds from source for Fedora 43/44/45.
3. **crates.io, AUR, and a Homebrew tap** — source installs.

The workflow in `.github/workflows/release.yml` builds and uploads the GitHub
Release assets. The AUR and Homebrew files for `v0.1.1` already carry the
source tarball checksum; for later releases, refresh them with the checksum
from the new tag.

Live locations:

- GitHub release: <https://github.com/oak443/complete-album/releases/tag/v0.1.1>
- Fedora COPR: <https://copr.fedorainfracloud.org/coprs/oak443/complete-album/>
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
git tag v0.1.1
git push origin v0.1.1
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

The same tag also submits a Fedora COPR build; see the next section.

## 2. Fedora COPR

The project is <https://copr.fedorainfracloud.org/coprs/oak443/complete-album/>.
It builds `complete-album.spec` from the Git tag using `.copr/Makefile`
(`make srpm`), with Rust crates vendored into the SRPM so the RPM build is
offline. The release workflow's `copr` job submits the build and waits for it.

Users enable and install with:

```bash
sudo dnf copr enable oak443/complete-album
sudo dnf install complete-album
```

The current chroots are Fedora 43, 44, and 45 on x86_64 and aarch64. To change
them, edit the COPR project settings or recreate it with a different
`copr_create_project` chroot list.

Manual submit after a tag:

```bash
python - <<'PY'
from copr.v3 import Client
c = Client.create_from_config_file()
b = c.build_proxy.create_from_scm(
    "oak443", "complete-album",
    "https://github.com/oak443/complete-album.git",
    committish="v0.1.1",
    spec="complete-album.spec",
    source_build_method="make_srpm",
)
print(b.id)
PY
```

## 3. crates.io

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

## 4. AUR

The source package is `packaging/aur/PKGBUILD`. After the GitHub release:

```bash
VERSION=0.1.1
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

## 5. Homebrew

The tap repository is
<https://github.com/oak443/homebrew-complete-album>; update
`Formula/complete-album.rb` there.

After the GitHub release:

```bash
curl -L "https://github.com/oak443/complete-album/archive/refs/tags/v0.1.1.tar.gz" \
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

- [ ] `Cargo.toml` version matches the tag (`v0.1.1` → `0.1.1`)
- [ ] `CHANGELOG.md` has the version's section
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `cargo package --allow-dirty`
- [ ] `git tag v0.1.1 && git push origin v0.1.1`
- [ ] GitHub Release assets and `SHA256SUMS` look right
- [ ] COPR build succeeded for all Fedora chroots
- [ ] `cargo publish`
- [ ] AUR checksum updated and `makepkg -si` tested
- [ ] Homebrew checksum updated and `brew test` passed
