//! complete-album: fill a ripped FLAC album with Picard-compatible
//! MusicBrainz tags and Cover Art Archive artwork.
//!
//! This is deliberately a standalone tool. It never re-encodes audio, and it
//! only replaces the tags it manages.

mod art;
mod mb;
mod tags;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;

use crate::mb::{MbClient, Track};

#[derive(Debug, Parser)]
#[command(
    name = "complete-album",
    version,
    about = "Write MusicBrainz tags and Cover Art Archive artwork into a FLAC album folder"
)]
struct Args {
    /// Album directory containing the FLAC files.
    album_dir: PathBuf,

    /// Print what would change; write nothing.
    #[arg(long)]
    dry_run: bool,

    /// Release MBID. Defaults to the album's MUSICBRAINZ_ALBUMID tag.
    #[arg(long)]
    release_id: Option<String>,

    /// Medium (disc) number for a multi-disc release.
    #[arg(long)]
    disc: Option<u32>,

    /// Do not fetch or write any artwork.
    #[arg(long)]
    no_art: bool,

    /// MusicBrainz web service base.
    #[arg(long, default_value = "https://musicbrainz.org/ws/2")]
    mb_base: String,

    /// Cover Art Archive base.
    #[arg(long, default_value = "https://coverartarchive.org")]
    caa_base: String,

    /// User-Agent sent to MusicBrainz and the Cover Art Archive. Put a real
    /// contact in it; MusicBrainz asks clients to identify themselves.
    #[arg(
        long,
        env = "COMPLETE_ALBUM_USER_AGENT",
        default_value = "complete-album/0.1 (https://example.invalid/complete-album)"
    )]
    user_agent: String,
}

fn main() {
    let args = Args::parse();
    match run(args) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::exit(1);
        }
    }
}

fn run(args: Args) -> Result<i32> {
    let album_dir = &args.album_dir;
    if !album_dir.is_dir() {
        bail!("{} is not a directory", album_dir.display());
    }

    let files = flac_files(album_dir)?;
    if files.is_empty() {
        bail!("no FLAC files in {}", album_dir.display());
    }

    let (release_id, existing_ids) = resolve_release_id(&files, args.release_id.as_deref())?;
    let client = MbClient::new(&args.mb_base, &args.user_agent);
    let release = client.release(&release_id)?;
    let disc_id = read_first_tag(&files[0], "MUSICBRAINZ_DISCID");
    let medium = mb::select_medium(&release, disc_id.as_deref(), args.disc, files.len())?;
    let assignments = assign_tracks(&files, medium)?;

    let album_tags = tags::album_tags(&release, medium, disc_id.as_deref());

    let fetched_art = if args.no_art {
        art::FetchedArt::default()
    } else {
        match art::fetch_art(
            &args.caa_base,
            &release_id,
            &args.user_agent,
            album_dir,
            args.dry_run,
        ) {
            Ok(fetched) => fetched,
            Err(error) => {
                eprintln!("warning: artwork was not fetched: {error:#}");
                art::FetchedArt::default()
            }
        }
    };

    if args.dry_run {
        print_header(&release, medium, &files, &existing_ids);
        println!("\nAlbum tags:");
        print_tags(&album_tags);
        for (index, (path, track)) in assignments.iter().enumerate() {
            println!(
                "\n{}. {}",
                index + 1,
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            print_tags(&tags::track_tags(medium, track));
        }
        println!("\nArtwork:");
        if let Some(front) = &fetched_art.front {
            println!("  would save {}", front.name);
        } else {
            println!("  no front cover found");
        }
        for image in &fetched_art.extra {
            println!("  would save {}", image.name);
        }
        for error in &fetched_art.errors {
            println!("  warning: {error}");
        }
        return Ok(if fetched_art.errors.is_empty() { 0 } else { 2 });
    }

    print_header(&release, medium, &files, &existing_ids);
    let mut failures = 0usize;
    for (path, track) in &assignments {
        let mut file_tags = album_tags.clone();
        file_tags.extend(tags::track_tags(medium, track));
        if let Err(error) = tags::write_file(path, &file_tags, fetched_art.front.as_ref(), false) {
            eprintln!("error: {}: {error:#}", path.display());
            failures += 1;
        }
    }

    println!("\nTags written: {} file(s)", assignments.len() - failures);
    if let Some(front) = &fetched_art.front {
        println!("Front cover: {} (embedded)", front.name);
    } else {
        println!("Front cover: none found");
    }
    if fetched_art.extra.is_empty() {
        println!("Extra artwork: none");
    } else {
        let names: Vec<&str> = fetched_art
            .extra
            .iter()
            .map(|image| image.name.as_str())
            .collect();
        println!("Extra artwork: {}", names.join(", "));
    }
    for error in &fetched_art.errors {
        eprintln!("warning: {error}");
    }

    Ok(if failures > 0 || !fetched_art.errors.is_empty() {
        2
    } else {
        0
    })
}

fn print_header(
    release: &mb::Release,
    medium: &mb::Medium,
    files: &[PathBuf],
    existing_ids: &[String],
) {
    println!("Release: {} — {}", release.artist_name(), release.title);
    println!("MBID:    {}", release.id);
    println!(
        "Disc:    {} ({} track(s) on the medium, {} FLAC file(s) here)",
        medium.position.unwrap_or(1),
        medium.track_total(),
        files.len()
    );
    if existing_ids.len() > 1 {
        println!(
            "Note:    the folder had {} distinct release IDs",
            existing_ids.len()
        );
    }
}

fn print_tags(tags: &tags::TagList) {
    for (key, values) in tags {
        println!("  {key}={}", values.join("; "));
    }
}

fn flac_files(album_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(album_dir)
        .with_context(|| format!("could not read {}", album_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("flac"))
        {
            files.push(path);
        }
    }
    files.sort_by_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    });
    Ok(files)
}

fn resolve_release_id(files: &[PathBuf], explicit: Option<&str>) -> Result<(String, Vec<String>)> {
    let mut ids = Vec::new();
    for file in files {
        if let Some(id) = tags::album_id(file)?
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }

    if let Some(explicit) = explicit.map(str::trim).filter(|id| !id.is_empty()) {
        return Ok((explicit.to_owned(), ids));
    }

    match ids.as_slice() {
        [] => bail!("no MUSICBRAINZ_ALBUMID tag found; pass --release-id or tag the files first"),
        [only] => Ok((only.clone(), ids)),
        _ => bail!(
            "the folder contains {} different MUSICBRAINZ_ALBUMID values; refusing to mix albums",
            ids.len()
        ),
    }
}

fn read_first_tag(path: &Path, key: &str) -> Option<String> {
    tags::read_vorbis(path)
        .ok()
        .and_then(|comments| comments.get(key).map(str::to_owned))
}

/// Match each file to one MusicBrainz track.
///
/// `TRACKNUMBER` is authoritative when present. Files without one fall back to
/// their sorted position. A track number that does not exist on the selected
/// medium is an error: writing the wrong title is worse than stopping.
fn assign_tracks<'a>(
    files: &[PathBuf],
    medium: &'a mb::Medium,
) -> Result<Vec<(PathBuf, &'a Track)>> {
    let mut used = HashSet::new();
    let mut assignments = Vec::with_capacity(files.len());

    for (index, path) in files.iter().enumerate() {
        let number = tags::track_number(path)?;
        let track = match number {
            Some(number) => {
                let found = medium.tracks.iter().position(|track| {
                    track.position == Some(number)
                        || track
                            .number
                            .as_deref()
                            .and_then(|value| value.parse::<u32>().ok())
                            == Some(number)
                });
                match found {
                    Some(found) => &medium.tracks[found],
                    None => bail!(
                        "{} has TRACKNUMBER={number}, but the selected medium has no track {number}",
                        path.display()
                    ),
                }
            }
            None => medium.tracks.get(index).with_context(|| {
                format!(
                    "{} has no TRACKNUMBER and the medium has only {} track(s)",
                    path.display(),
                    medium.tracks.len()
                )
            })?,
        };

        if !used.insert(track.id.clone()) {
            bail!(
                "{} maps to the same MusicBrainz track as another file ({}); refusing",
                path.display(),
                track.id
            );
        }
        assignments.push((path.clone(), track));
    }

    Ok(assignments)
}
