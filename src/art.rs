//! Cover Art Archive client.
//!
//! The front cover has a one-shot endpoint; back covers and booklet pages come
//! from the release's typed-image manifest. Everything here is best-effort:
//! a missing image never stops the metadata write.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use url::Url;

use crate::tags::ImageKind;

const MAX_BYTES: usize = 30 * 1024 * 1024;
const MAX_EXTRA_IMAGES: usize = 60;

/// One image fetched from the archive.
pub struct ArtImage {
    pub name: String,
    pub bytes: Vec<u8>,
    pub kind: ImageKind,
}

/// The complete artwork package for one release.
#[derive(Default)]
pub struct FetchedArt {
    pub front: Option<ArtImage>,
    /// Back cover and booklet files, in the order they were saved.
    pub extra: Vec<ArtImage>,
    /// Human-readable problems; these do not abort the tag write.
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(default)]
    images: Vec<ManifestImage>,
}

#[derive(Debug, Deserialize)]
struct ManifestImage {
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    image: Option<String>,
}

/// Fetch the front cover, back cover, and booklet pages for `mbid`.
///
/// `dry_run` means "do not write files", but the images are still fetched so
/// the summary is honest about what would be saved.
pub fn fetch_art(
    caa_base: &str,
    mbid: &str,
    user_agent: &str,
    album_dir: &Path,
    dry_run: bool,
) -> Result<FetchedArt> {
    let base = caa_base.trim_end_matches('/');
    let mut art = FetchedArt::default();

    let front_url = format!("{base}/release/{}/front", encode(mbid));
    match fetch(&front_url, user_agent) {
        Ok(Some(bytes)) => match image_kind(&bytes) {
            Some(kind) => {
                let image = ArtImage {
                    name: format!("cover.{}", kind.extension()),
                    bytes,
                    kind,
                };
                if !dry_run && let Err(error) = write_image(album_dir, &image) {
                    art.errors.push(error.to_string());
                }
                art.front = Some(image);
            }
            None => art.errors.push(format!(
                "front cover at {front_url} is not a JPEG/PNG/GIF image"
            )),
        },
        Ok(None) => {}
        Err(error) => art.errors.push(format!("front cover: {error}")),
    }

    let manifest_url = format!("{base}/release/{}", encode(mbid));
    let manifest = match fetch(&manifest_url, user_agent) {
        Ok(Some(bytes)) => match serde_json::from_slice::<Manifest>(&bytes) {
            Ok(manifest) => manifest,
            Err(error) => {
                art.errors
                    .push(format!("cover-art manifest is not valid JSON: {error}"));
                Manifest { images: vec![] }
            }
        },
        Ok(None) => Manifest { images: vec![] },
        Err(error) => {
            art.errors.push(format!("cover-art manifest: {error}"));
            Manifest { images: vec![] }
        }
    };

    let mut have_back = false;
    let mut booklet = 0usize;
    let mut fetched = 0usize;
    for image in manifest.images {
        let Some(url) = image.image.as_deref() else {
            continue;
        };
        let types: Vec<String> = image
            .types
            .iter()
            .map(|kind| kind.to_ascii_lowercase())
            .collect();
        let stem = if types.iter().any(|kind| kind == "back") && !have_back {
            "back".to_owned()
        } else if types.iter().any(|kind| kind == "booklet") {
            booklet += 1;
            format!("booklet-{booklet:02}")
        } else {
            continue;
        };

        if fetched >= MAX_EXTRA_IMAGES {
            art.errors.push(format!(
                "more than {MAX_EXTRA_IMAGES} back/booklet images; kept the first {MAX_EXTRA_IMAGES}"
            ));
            break;
        }
        if !image_url_is_allowed(url, base) {
            art.errors.push(format!(
                "cover-art manifest named an image outside the archive's hosts; skipped: {url}"
            ));
            continue;
        }
        fetched += 1;

        match fetch(url, user_agent) {
            Ok(Some(bytes)) => match image_kind(&bytes) {
                Some(kind) => {
                    let image = ArtImage {
                        name: format!("{stem}.{}", kind.extension()),
                        bytes,
                        kind,
                    };
                    if !dry_run && let Err(error) = write_image(album_dir, &image) {
                        art.errors.push(error.to_string());
                        continue;
                    }
                    if stem == "back" {
                        have_back = true;
                    }
                    art.extra.push(image);
                }
                None => art
                    .errors
                    .push(format!("{stem}: response is not an image or PDF")),
            },
            Ok(None) => art
                .errors
                .push(format!("{stem}: the image URL returned 404")),
            Err(error) => art.errors.push(format!("{stem}: {error}")),
        }
    }

    Ok(art)
}

fn write_image(album_dir: &Path, image: &ArtImage) -> Result<()> {
    let path = album_dir.join(&image.name);
    std::fs::write(&path, &image.bytes)
        .with_context(|| format!("could not write {}", path.display()))
}

/// `Some(bytes)` for 200, `None` for 404, `Err` for anything else.
fn fetch(url: &str, user_agent: &str) -> Result<Option<Vec<u8>>> {
    match ureq::get(url).header("User-Agent", user_agent).call() {
        Ok(mut response) => {
            let body = response
                .body_mut()
                .with_config()
                .limit(MAX_BYTES as u64)
                .read_to_vec()
                .with_context(|| format!("could not read {url}"))?;
            Ok(Some(body))
        }
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(error) => bail!("{error}"),
    }
}

/// Recognise the image formats we can embed and the PDF booklets we can save.
pub fn image_kind(bytes: &[u8]) -> Option<ImageKind> {
    if bytes.starts_with(b"\xff\xd8\xff") {
        Some(ImageKind::Jpeg)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageKind::Png)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(ImageKind::Gif)
    } else if bytes.starts_with(b"%PDF-") {
        Some(ImageKind::Pdf)
    } else {
        None
    }
}

/// True for an http(s) URL on the archive's hosts (or the configured base,
/// which lets a local test server stand in for the archive).
pub fn image_url_is_allowed(url: &str, base: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return false;
    }
    let Some(host) = parsed.host_str().map(|host| host.to_ascii_lowercase()) else {
        return false;
    };
    let base_host = Url::parse(base)
        .ok()
        .and_then(|base| base.host_str().map(|host| host.to_ascii_lowercase()));
    let allowed = ["coverartarchive.org", "archive.org"];
    allowed
        .iter()
        .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
        || base_host.is_some_and(|base| host == base || host.ends_with(&format!(".{base}")))
}

fn encode(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_formats_we_use() {
        assert_eq!(image_kind(b"\xff\xd8\xff\xe0"), Some(ImageKind::Jpeg));
        assert_eq!(image_kind(b"\x89PNG\r\n\x1a\n"), Some(ImageKind::Png));
        assert_eq!(image_kind(b"GIF89a"), Some(ImageKind::Gif));
        assert_eq!(image_kind(b"%PDF-1.7"), Some(ImageKind::Pdf));
        assert_eq!(image_kind(b"<html>"), None);
    }

    #[test]
    fn allows_archive_hosts_and_configured_base() {
        assert!(image_url_is_allowed(
            "https://coverartarchive.org/release/x/front",
            "https://coverartarchive.org"
        ));
        assert!(image_url_is_allowed(
            "https://ia800.example.archive.org/x.jpg",
            "https://coverartarchive.org"
        ));
        assert!(image_url_is_allowed(
            "http://127.0.0.1:1234/back.jpg",
            "http://127.0.0.1:1234"
        ));
        assert!(!image_url_is_allowed(
            "file:///etc/passwd",
            "https://coverartarchive.org"
        ));
        assert!(!image_url_is_allowed(
            "http://192.168.1.1/x.jpg",
            "https://coverartarchive.org"
        ));
    }
}
