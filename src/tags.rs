//! Picard-compatible tag mapping and FLAC writing.
//!
//! Only the keys listed here are managed. All other Vorbis comments
//! (ReplayGain, comments, user tags) and all pictures except the front cover
//! are preserved.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Context, Result};
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::flac::FlacFile;
use lofty::ogg::OggPictureStorage;
use lofty::ogg::tag::VorbisComments;
use lofty::picture::{MimeType, Picture, PictureType};

use crate::mb::{
    Medium, Release, Track, credit_artist_ids, credit_artist_names, render_credit, sort_credit,
};

/// A tag key and its values, in the order they should be written. Vorbis
/// comments allow the same key more than once, which is how multi-value tags
/// such as `ARTIST` and `ISRC` work.
pub type TagList = Vec<(String, Vec<String>)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Jpeg,
    Png,
    Gif,
    Pdf,
}

impl ImageKind {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::Pdf => "pdf",
        }
    }

    /// `None` for PDF, which cannot be embedded as a FLAC picture.
    pub fn mime(self) -> Option<MimeType> {
        match self {
            Self::Jpeg => Some(MimeType::Jpeg),
            Self::Png => Some(MimeType::Png),
            Self::Gif => Some(MimeType::Gif),
            Self::Pdf => None,
        }
    }
}

/// Album-level tags written to every file.
pub fn album_tags(release: &Release, medium: &Medium, disc_id: Option<&str>) -> TagList {
    let mut tags: TagList = Vec::new();

    push(&mut tags, "MUSICBRAINZ_ALBUMID", &release.id);
    if let Some(group) = &release.release_group {
        push(&mut tags, "MUSICBRAINZ_RELEASEGROUPID", &group.id);
        if let Some(date) = group.first_release_date.as_deref() {
            push(&mut tags, "ORIGINALDATE", date);
            push(&mut tags, "ORIGINALYEAR", &date[..date.len().min(4)]);
        }
    }

    let credit = &release.artist_credit;
    for id in credit_artist_ids(credit) {
        push(&mut tags, "MUSICBRAINZ_ALBUMARTISTID", &id);
    }
    push(&mut tags, "ALBUMARTIST", &render_credit(credit));
    for name in credit_artist_names(credit) {
        push(&mut tags, "ALBUMARTISTS", &name);
    }
    push(&mut tags, "ALBUMARTISTSORT", &sort_credit(credit));
    push(&mut tags, "ALBUM", &release.title);

    if let Some(date) = release.date.as_deref() {
        push(&mut tags, "DATE", date);
    }
    if let Some(country) = release.country.as_deref() {
        push(&mut tags, "RELEASECOUNTRY", country);
    }
    if let Some(status) = release.status.as_deref() {
        push(&mut tags, "RELEASESTATUS", &status.to_lowercase());
    }
    if let Some(group) = &release.release_group {
        if let Some(kind) = group.primary_type.as_deref() {
            push(&mut tags, "RELEASETYPE", &kind.to_lowercase());
        }
        for kind in &group.secondary_types {
            push(&mut tags, "RELEASETYPE", &kind.to_lowercase());
        }
    }

    for info in &release.label_info {
        if let Some(label) = info.label.as_ref().and_then(|label| label.name.as_deref()) {
            push_unique(&mut tags, "LABEL", label);
        }
        if let Some(catalog) = info.catalog_number.as_deref() {
            push_unique(&mut tags, "CATALOGNUMBER", catalog);
        }
    }
    if let Some(barcode) = release.barcode.as_deref() {
        push(&mut tags, "BARCODE", barcode);
    }
    if let Some(asin) = release.asin.as_deref() {
        push(&mut tags, "ASIN", asin);
    }
    if let Some(text) = &release.text_representation {
        if let Some(script) = text.script.as_deref() {
            push(&mut tags, "SCRIPT", script);
        }
        if let Some(language) = text.language.as_deref() {
            push(&mut tags, "LANGUAGE", language);
        }
    }

    if release.is_various_artists() {
        push(&mut tags, "COMPILATION", "1");
    }
    if let Some(format) = medium.format.as_deref() {
        push(&mut tags, "MEDIA", format);
    }
    if let Some(subtitle) = medium.title.as_deref() {
        push(&mut tags, "DISCSUBTITLE", subtitle);
    }
    if let Some(position) = medium.position {
        push(&mut tags, "DISCNUMBER", &position.to_string());
    }
    let total_discs = release.media.len().max(1).to_string();
    push(&mut tags, "DISCTOTAL", &total_discs);
    // Platterpus and several players use this spelling; Picard uses DISCTOTAL.
    push(&mut tags, "TOTALDISCS", &total_discs);

    // Keep the disc ID the rip actually read when it belongs to this medium;
    // a release can carry many disc IDs, and the first one is not necessarily
    // the disc in the user's hand.
    let chosen_disc_id = disc_id
        .filter(|id| medium.has_disc_id(id))
        .or_else(|| medium.discs.first().map(|disc| disc.id.as_str()));
    if let Some(disc_id) = chosen_disc_id {
        push(&mut tags, "MUSICBRAINZ_DISCID", disc_id);
    }
    if let Some(genre) = release.genre() {
        push(&mut tags, "GENRE", genre);
    }

    tags
}

/// Track-level tags for one file.
pub fn track_tags(medium: &Medium, track: &Track) -> TagList {
    let mut tags: TagList = Vec::new();

    push(&mut tags, "MUSICBRAINZ_TRACKID", &track.id);
    if let Some(recording) = &track.recording {
        push(&mut tags, "MUSICBRAINZ_RECORDINGID", &recording.id);
    }

    let credit = track.track_artist_credit();
    for id in credit_artist_ids(credit) {
        push(&mut tags, "MUSICBRAINZ_ARTISTID", &id);
    }
    push(&mut tags, "ARTIST", &render_credit(credit));
    for name in credit_artist_names(credit) {
        push(&mut tags, "ARTISTS", &name);
    }
    push(&mut tags, "ARTISTSORT", &sort_credit(credit));

    if let Some(title) = track.recording_title() {
        push(&mut tags, "TITLE", title);
    }
    if let Some(recording) = &track.recording {
        for isrc in &recording.isrcs {
            push(&mut tags, "ISRC", isrc);
        }
    }
    if let Some(position) = track.position {
        push(&mut tags, "TRACKNUMBER", &position.to_string());
    }
    push(&mut tags, "TRACKTOTAL", &medium.track_total().to_string());

    if let Some(work) = track.work() {
        push(&mut tags, "MUSICBRAINZ_WORKID", &work.id);
        if let Some(title) = work.title.as_deref() {
            push(&mut tags, "WORK", title);
        }
    }

    tags
}

fn push(tags: &mut TagList, key: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    if let Some((_, values)) = tags.iter_mut().find(|(existing, _)| existing == key) {
        values.push(value.to_owned());
    } else {
        tags.push((key.to_owned(), vec![value.to_owned()]));
    }
}

fn push_unique(tags: &mut TagList, key: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    if let Some((_, values)) = tags.iter_mut().find(|(existing, _)| existing == key) {
        if !values.iter().any(|existing| existing == value) {
            values.push(value.to_owned());
        }
    } else {
        tags.push((key.to_owned(), vec![value.to_owned()]));
    }
}

/// Read one FLAC's Vorbis comments.
pub fn read_vorbis(path: &Path) -> Result<VorbisComments> {
    let mut file =
        File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let flac = FlacFile::read_from(&mut BufReader::new(&mut file), ParseOptions::new())
        .with_context(|| format!("could not read FLAC tags from {}", path.display()))?;
    Ok(flac.vorbis_comments().cloned().unwrap_or_default())
}

pub fn album_id(path: &Path) -> Result<Option<String>> {
    Ok(read_vorbis(path)?
        .get("MUSICBRAINZ_ALBUMID")
        .map(str::to_owned))
}

pub fn track_number(path: &Path) -> Result<Option<u32>> {
    Ok(read_vorbis(path)?
        .get("TRACKNUMBER")
        .and_then(|value| value.trim().parse().ok()))
}

/// Write `tags` and optionally embed `front` into one FLAC.
///
/// The file is read once, its managed keys are removed, the new values are
/// appended, and the result is saved once. Audio frames are untouched.
pub fn write_file(
    path: &Path,
    tags: &TagList,
    front: Option<&crate::art::ArtImage>,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        return Ok(());
    }
    let mut file =
        File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let mut flac = FlacFile::read_from(&mut BufReader::new(&mut file), ParseOptions::new())
        .with_context(|| format!("could not read {}", path.display()))?;

    let mut comments = flac.vorbis_comments().cloned().unwrap_or_default();
    for (key, _) in tags {
        let _ = comments.remove(key).count();
    }
    for (key, values) in tags {
        for value in values {
            if !value.is_empty() {
                comments.push(key.clone(), value.clone());
            }
        }
    }
    flac.set_vorbis_comments(comments);

    if let Some(image) = front
        && let Some(mime) = image.kind.mime()
    {
        flac.remove_picture_type(PictureType::CoverFront);
        let picture = Picture::unchecked(image.bytes.clone())
            .pic_type(PictureType::CoverFront)
            .mime_type(mime)
            .description("Front cover")
            .build();
        flac.insert_picture(picture, None)
            .with_context(|| format!("could not embed the front cover in {}", path.display()))?;
    }

    flac.save_to_path(path, WriteOptions::default())
        .with_context(|| format!("could not save {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mb::{Artist, ArtistCredit, ReleaseGroup};

    fn artist() -> ArtistCredit {
        ArtistCredit {
            name: Some("The Police".into()),
            joinphrase: None,
            artist: Some(Artist {
                id: "artist-id".into(),
                name: "The Police".into(),
                sort_name: Some("Police, The".into()),
            }),
        }
    }

    fn release() -> Release {
        Release {
            id: "release-id".into(),
            title: "Every Breath You Take".into(),
            status: Some("Official".into()),
            date: Some("1995-09-12".into()),
            country: Some("US".into()),
            barcode: Some("123".into()),
            asin: None,
            artist_credit: vec![artist()],
            release_group: Some(ReleaseGroup {
                id: "group-id".into(),
                primary_type: Some("Album".into()),
                secondary_types: vec!["Compilation".into()],
                first_release_date: Some("1986-11-03".into()),
            }),
            label_info: vec![],
            media: vec![Medium {
                position: Some(1),
                format: Some("CD".into()),
                title: None,
                track_count: Some(14),
                tracks: vec![],
                discs: vec![crate::mb::Disc {
                    id: "disc-id".into(),
                }],
            }],
            tags: vec![],
            text_representation: None,
        }
    }

    #[test]
    fn album_tags_include_the_picard_ids_and_dates() {
        let release = release();
        let tags = album_tags(&release, &release.media[0], None);
        let get = |key: &str| {
            tags.iter()
                .find(|(existing, _)| existing == key)
                .map(|(_, values)| values.join(";"))
                .unwrap_or_default()
        };
        assert_eq!(get("MUSICBRAINZ_ALBUMID"), "release-id");
        assert_eq!(get("MUSICBRAINZ_RELEASEGROUPID"), "group-id");
        assert_eq!(get("MUSICBRAINZ_ALBUMARTISTID"), "artist-id");
        assert_eq!(get("ORIGINALDATE"), "1986-11-03");
        assert_eq!(get("ORIGINALYEAR"), "1986");
        assert_eq!(get("RELEASETYPE"), "album;compilation");
        assert_eq!(get("DISCTOTAL"), "1");
    }

    #[test]
    fn album_tags_keep_a_disc_id_that_belongs_to_the_medium() {
        let release = release();
        let medium = &release.media[0];
        let get = |disc_id: Option<&str>| {
            album_tags(&release, medium, disc_id)
                .into_iter()
                .find(|(key, _)| key == "MUSICBRAINZ_DISCID")
                .map(|(_, values)| values.join(";"))
                .unwrap_or_default()
        };
        assert_eq!(get(Some("disc-id")), "disc-id");
        assert_eq!(get(Some("another-disc")), "disc-id");
    }

    #[test]
    fn track_tags_include_track_and_recording_ids() {
        let medium = release().media.remove(0);
        let track = Track {
            id: "track-id".into(),
            number: Some("1".into()),
            title: Some("Roxanne".into()),
            position: Some(1),
            artist_credit: vec![artist()],
            recording: Some(crate::mb::Recording {
                id: "recording-id".into(),
                title: Some("Roxanne".into()),
                isrcs: vec!["GBAAM0201086".into(), "GBAAM7800003".into()],
                artist_credit: vec![artist()],
                relations: vec![],
            }),
        };
        let tags = track_tags(&medium, &track);
        let get = |key: &str| {
            tags.iter()
                .find(|(existing, _)| existing == key)
                .map(|(_, values)| values.join(";"))
                .unwrap_or_default()
        };
        assert_eq!(get("MUSICBRAINZ_TRACKID"), "track-id");
        assert_eq!(get("MUSICBRAINZ_RECORDINGID"), "recording-id");
        assert_eq!(get("MUSICBRAINZ_ARTISTID"), "artist-id");
        assert_eq!(get("ISRC"), "GBAAM0201086;GBAAM7800003");
        assert_eq!(get("TRACKNUMBER"), "1");
        assert_eq!(get("TRACKTOTAL"), "14");
    }
}
