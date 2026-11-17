//! MusicBrainz Web Service v2 client and the subset of the release JSON this
//! tool needs.
//!
//! The API needs no key for reads, but it requires a descriptive User-Agent and
//! asks clients to stay at or below one request per second. This tool makes one
//! release request per run, so the limit is not a concern.

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The release fields we request. `recordings` makes MusicBrainz return
/// `media[].tracks[]` (not `medium-list[].track-list[]`), which is what the
/// types below expect.
pub const INCLUDES: &str = concat!(
    "artists+artist-credits+recordings+labels+media+tags+isrcs+discids+",
    "release-groups+work-rels+recording-rels"
);

/// A release, as returned by `GET /ws/2/release/{mbid}?inc=...&fmt=json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub barcode: Option<String>,
    #[serde(default)]
    pub asin: Option<String>,
    #[serde(rename = "artist-credit", default)]
    pub artist_credit: Vec<ArtistCredit>,
    #[serde(rename = "release-group", default)]
    pub release_group: Option<ReleaseGroup>,
    #[serde(rename = "label-info", default)]
    pub label_info: Vec<LabelInfo>,
    #[serde(default)]
    pub media: Vec<Medium>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(rename = "text-representation", default)]
    pub text_representation: Option<TextRepresentation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArtistCredit {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub joinphrase: Option<String>,
    #[serde(default)]
    pub artist: Option<Artist>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    #[serde(rename = "sort-name", default)]
    pub sort_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseGroup {
    pub id: String,
    #[serde(rename = "primary-type", default)]
    pub primary_type: Option<String>,
    #[serde(rename = "secondary-types", default)]
    pub secondary_types: Vec<String>,
    #[serde(rename = "first-release-date", default)]
    pub first_release_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LabelInfo {
    #[serde(rename = "catalog-number", default)]
    pub catalog_number: Option<String>,
    #[serde(default)]
    pub label: Option<Label>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Label {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Medium {
    #[serde(default)]
    pub position: Option<u32>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(rename = "track-count", default)]
    pub track_count: Option<u32>,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub discs: Vec<Disc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Disc {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Track {
    pub id: String,
    #[serde(default)]
    pub number: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub position: Option<u32>,
    #[serde(rename = "artist-credit", default)]
    pub artist_credit: Vec<ArtistCredit>,
    #[serde(default)]
    pub recording: Option<Recording>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Recording {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub isrcs: Vec<String>,
    #[serde(rename = "artist-credit", default)]
    pub artist_credit: Vec<ArtistCredit>,
    #[serde(default)]
    pub relations: Vec<Relation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Relation {
    #[serde(rename = "target-type", default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub work: Option<Work>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Work {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(default)]
    pub count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextRepresentation {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
}

/// Read-only MusicBrainz client.
pub struct MbClient {
    base: String,
    user_agent: String,
}

impl MbClient {
    pub fn new(base: impl Into<String>, user_agent: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            user_agent: user_agent.into(),
        }
    }

    /// Fetch one release by MBID. A missing release is an error (this tool
    /// only runs for known releases).
    pub fn release(&self, mbid: &str) -> Result<Release> {
        if mbid.trim().is_empty() {
            bail!("the release MBID is empty");
        }
        let url = format!(
            "{}/release/{}?inc={}&fmt=json",
            self.base,
            urlencode(mbid.trim()),
            INCLUDES
        );
        let mut response = ureq::get(&url)
            .header("User-Agent", &self.user_agent)
            .call()
            .with_context(|| format!("MusicBrainz request failed: {url}"))?;
        let body = response
            .body_mut()
            .with_config()
            .limit(10 * 1024 * 1024)
            .read_to_vec()
            .with_context(|| format!("could not read the MusicBrainz response for {mbid}"))?;
        serde_json::from_slice(&body).with_context(|| {
            format!("MusicBrainz returned JSON this tool could not parse for {mbid}")
        })
    }
}

/// Percent-encode a path segment. MBIDs are UUIDs, so this is mostly a
/// guard against a user passing something that would change the URL path.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

impl Release {
    /// The release-level artist credit as one display string.
    pub fn artist_name(&self) -> String {
        render_credit(&self.artist_credit)
    }

    /// The highest-voted MusicBrainz tag, used as a best-effort genre.
    pub fn genre(&self) -> Option<&str> {
        self.tags
            .iter()
            .max_by_key(|tag| tag.count)
            .map(|tag| tag.name.as_str())
    }

    /// true when the release artist is MusicBrainz's "Various Artists".
    pub fn is_various_artists(&self) -> bool {
        self.artist_credit.iter().any(|credit| {
            credit
                .artist
                .as_ref()
                .is_some_and(|artist| artist.id == "89ad4ac3-39f7-470e-963a-56509c546377")
        })
    }
}

impl Medium {
    pub fn track_total(&self) -> u32 {
        self.track_count.unwrap_or(self.tracks.len() as u32)
    }

    pub fn has_disc_id(&self, disc_id: &str) -> bool {
        self.discs.iter().any(|disc| disc.id == disc_id)
    }
}

impl Track {
    pub fn recording_title(&self) -> Option<&str> {
        self.recording
            .as_ref()
            .and_then(|recording| recording.title.as_deref())
            .or(self.title.as_deref())
    }

    pub fn track_artist_credit(&self) -> &[ArtistCredit] {
        if self.artist_credit.is_empty() {
            self.recording
                .as_ref()
                .map(|recording| recording.artist_credit.as_slice())
                .unwrap_or(&[])
        } else {
            &self.artist_credit
        }
    }

    pub fn work(&self) -> Option<&Work> {
        self.recording
            .as_ref()?
            .relations
            .iter()
            .find(|relation| {
                relation.target_type.as_deref() == Some("work") && relation.work.is_some()
            })?
            .work
            .as_ref()
    }
}

/// Render MusicBrainz's artist-credit list ("The Police", "A feat. B").
pub fn render_credit(credit: &[ArtistCredit]) -> String {
    let mut out = String::new();
    for item in credit {
        if let Some(name) = item.name.as_deref().filter(|name| !name.is_empty()) {
            out.push_str(name);
        } else if let Some(artist) = &item.artist {
            out.push_str(&artist.name);
        }
        if let Some(join) = item.joinphrase.as_deref() {
            out.push_str(join);
        }
    }
    out.trim().to_owned()
}

/// The join of each credited artist's sort name, for `ARTISTSORT` /
/// `ALBUMARTISTSORT`.
pub fn sort_credit(credit: &[ArtistCredit]) -> String {
    let mut out = String::new();
    for item in credit {
        if let Some(artist) = &item.artist {
            out.push_str(artist.sort_name.as_deref().unwrap_or(&artist.name));
        }
        if let Some(join) = item.joinphrase.as_deref() {
            out.push_str(join);
        }
    }
    out.trim().to_owned()
}

/// Every artist MBID in a credit, in order.
pub fn credit_artist_ids(credit: &[ArtistCredit]) -> Vec<String> {
    credit
        .iter()
        .filter_map(|item| item.artist.as_ref().map(|artist| artist.id.clone()))
        .collect()
}

/// Every credited artist's display name, in order.
pub fn credit_artist_names(credit: &[ArtistCredit]) -> Vec<String> {
    credit
        .iter()
        .filter_map(|item| {
            item.artist
                .as_ref()
                .map(|artist| artist.name.clone())
                .or_else(|| item.name.clone())
        })
        .collect()
}

/// Pick the medium that belongs to the album folder.
///
/// Order: an explicit `--disc N`, then the folder's MusicBrainz disc ID, then a
/// unique track-count match, then a single-medium release. Anything ambiguous
/// is refused rather than guessed.
pub fn select_medium<'a>(
    release: &'a Release,
    disc_id: Option<&str>,
    want_disc: Option<u32>,
    flac_count: usize,
) -> Result<&'a Medium> {
    if release.media.is_empty() {
        bail!("MusicBrainz returned no media for this release");
    }

    if let Some(position) = want_disc {
        if let Some(medium) = release
            .media
            .iter()
            .find(|medium| medium.position == Some(position))
        {
            return Ok(medium);
        }
        bail!(
            "the release has no medium {position}; choose 1..{}",
            release.media.len()
        );
    }

    if let Some(disc_id) = disc_id.map(str::trim).filter(|id| !id.is_empty()) {
        let matches: Vec<&Medium> = release
            .media
            .iter()
            .filter(|medium| medium.has_disc_id(disc_id))
            .collect();
        match matches.as_slice() {
            [only] => return Ok(only),
            [] => {}
            _ => bail!("the disc ID {disc_id} appears on more than one medium; pass --disc N"),
        }
    }

    let by_tracks: Vec<&Medium> = release
        .media
        .iter()
        .filter(|medium| medium.tracks.len() == flac_count)
        .collect();
    if let [only] = by_tracks.as_slice() {
        return Ok(only);
    }

    if release.media.len() == 1 {
        return Ok(&release.media[0]);
    }

    let described: Vec<String> = release
        .media
        .iter()
        .map(|medium| {
            format!(
                "disc {} ({} track(s))",
                medium.position.unwrap_or(0),
                medium.tracks.len()
            )
        })
        .collect();
    bail!(
        "could not tell which medium this folder is: {}. Pass --disc N",
        described.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credit(name: &str, sort: &str) -> ArtistCredit {
        ArtistCredit {
            name: Some(name.to_owned()),
            joinphrase: None,
            artist: Some(Artist {
                id: "id".to_owned(),
                name: name.to_owned(),
                sort_name: Some(sort.to_owned()),
            }),
        }
    }

    #[test]
    fn render_credit_uses_names_and_join_phrases() {
        let mut a = credit("A", "A, The");
        a.joinphrase = Some(" feat. ".to_owned());
        let b = credit("B", "B, The");
        assert_eq!(render_credit(&[a, b]), "A feat. B");
    }

    #[test]
    fn sort_credit_uses_sort_names() {
        assert_eq!(
            sort_credit(&[credit("The Police", "Police, The")]),
            "Police, The"
        );
    }

    #[test]
    fn medium_selection_prefers_disc_id() {
        let release = Release {
            id: "r".into(),
            title: "T".into(),
            status: None,
            date: None,
            country: None,
            barcode: None,
            asin: None,
            artist_credit: vec![],
            release_group: None,
            label_info: vec![],
            media: vec![
                Medium {
                    position: Some(1),
                    format: None,
                    title: None,
                    track_count: Some(1),
                    tracks: vec![],
                    discs: vec![Disc { id: "one".into() }],
                },
                Medium {
                    position: Some(2),
                    format: None,
                    title: None,
                    track_count: Some(1),
                    tracks: vec![],
                    discs: vec![Disc { id: "two".into() }],
                },
            ],
            tags: vec![],
            text_representation: None,
        };
        let medium = select_medium(&release, Some("two"), None, 1).unwrap();
        assert_eq!(medium.position, Some(2));
    }

    #[test]
    fn medium_selection_refuses_ambiguity() {
        let release = Release {
            id: "r".into(),
            title: "T".into(),
            status: None,
            date: None,
            country: None,
            barcode: None,
            asin: None,
            artist_credit: vec![],
            release_group: None,
            label_info: vec![],
            media: vec![
                Medium {
                    position: Some(1),
                    format: None,
                    title: None,
                    track_count: None,
                    tracks: vec![],
                    discs: vec![],
                },
                Medium {
                    position: Some(2),
                    format: None,
                    title: None,
                    track_count: None,
                    tracks: vec![],
                    discs: vec![],
                },
            ],
            tags: vec![],
            text_representation: None,
        };
        assert!(select_medium(&release, None, None, 5).is_err());
        assert_eq!(
            select_medium(&release, None, Some(2), 5).unwrap().position,
            Some(2)
        );
    }
}
