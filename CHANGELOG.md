# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-11-17

### Added

- Fedora RPM spec (`complete-album.spec`).
- COPR source build via `.copr/Makefile` with vendored Rust crates.
- GitHub webhook-triggered COPR builds, with no COPR token stored in GitHub.
- Fedora install instructions via `dnf copr enable oak443/complete-album`.

## [0.1.0] - 2026-11-17

### Added

- `complete-album` CLI for FLAC album folders.
- MusicBrainz release lookup by `MUSICBRAINZ_ALBUMID` or `--release-id`.
- Multi-disc medium selection by disc ID, explicit `--disc`, or unique track
  count.
- Picard-compatible album and track tags, including MusicBrainz entity IDs,
  artist sort names, release-group and work IDs, ISRCs, and release metadata.
- Cover Art Archive front cover embedding plus `cover.*`, `back.*`, and
  `booklet-NN.*` files; PDF booklets are saved as PDF.
- `--dry-run`, `--no-art`, and local test-server support through
  `--mb-base` / `--caa-base`.
- Unit and integration tests.
