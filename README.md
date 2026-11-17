# complete-album

Fill a ripped FLAC album folder with Picard-compatible MusicBrainz tags and
Cover Art Archive artwork.

One command:

```bash
complete-album /path/to/album
```

It reads the album's `MUSICBRAINZ_ALBUMID`, fetches the release from
MusicBrainz, writes the full tag set Picard expects, and saves the artwork:

- front cover embedded in every FLAC, plus `cover.jpg` / `cover.png`
- `back.jpg` / `back.png`
- `booklet-01.jpg`, `booklet-02.jpg`, … and `booklet-NN.pdf` for PDF booklets

Audio is never re-encoded. Only the tags this tool manages are replaced;
ReplayGain, comments, user tags and non-front pictures are preserved.

## Install

Prebuilt binaries are attached to each
[GitHub Release](https://github.com/oak443/complete-album/releases/latest)
(Linux musl, macOS, Windows) together with `SHA256SUMS`.

```bash
# Homebrew tap
brew install oak443/complete-album/complete-album

# After the crates.io release
cargo install complete-album

# From a checkout
cargo install --path .

# Or build locally
cargo build --release
./target/release/complete-album --help
```

The result is a single binary with no Python or runtime dependencies.
Packaging files for AUR and Homebrew are under `packaging/`.

## Usage

```bash
# See what would change, write nothing
complete-album --dry-run /path/to/album

# Write tags and artwork
complete-album --user-agent 'complete-album/0.1 (you@example.com)' /path/to/album

# Override the release, or select a disc of a multi-disc set
complete-album --release-id <mbid> --disc 2 /path/to/album

# Metadata only
complete-album --no-art /path/to/album
```

MusicBrainz asks every client to send a descriptive `User-Agent` with a real
contact. Set `--user-agent` or the `COMPLETE_ALBUM_USER_AGENT` environment
variable.

Exit codes: `0` success, `1` refusal/usage error, `2` partial success (some
files or artwork failed; details on stderr).

## What gets written

Album-level: `MUSICBRAINZ_ALBUMID`, `MUSICBRAINZ_RELEASEGROUPID`,
`MUSICBRAINZ_ALBUMARTISTID`, `ALBUMARTIST`, `ALBUMARTISTS`,
`ALBUMARTISTSORT`, `ALBUM`, `DATE`, `ORIGINALDATE`, `ORIGINALYEAR`,
`RELEASECOUNTRY`, `RELEASESTATUS`, `RELEASETYPE`, `LABEL`,
`CATALOGNUMBER`, `BARCODE`, `ASIN`, `SCRIPT`, `LANGUAGE`, `COMPILATION`,
`MEDIA`, `DISCSUBTITLE`, `DISCNUMBER`, `DISCTOTAL`, `TOTALDISCS`,
`MUSICBRAINZ_DISCID`, `GENRE`.

Track-level: `MUSICBRAINZ_TRACKID`, `MUSICBRAINZ_RECORDINGID`,
`MUSICBRAINZ_ARTISTID`, `ARTIST`, `ARTISTS`, `ARTISTSORT`, `TITLE`,
`ISRC`, `TRACKNUMBER`, `TRACKTOTAL`, and `MUSICBRAINZ_WORKID` / `WORK`
when MusicBrainz has a work relation.

## Scope

MVP boundaries:

- FLAC only; the release must already exist in MusicBrainz.
- No composer/lyricist/performer relationship tags yet.
- PDF booklets are saved as PDF, not converted to images.
- Managed tags are overwritten from MusicBrainz; everything else is kept.
- No renaming, moving, or audio re-encoding.
