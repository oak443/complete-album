use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::flac::FlacFile;
use lofty::ogg::OggPictureStorage;
use lofty::ogg::tag::VorbisComments;
use lofty::picture::PictureType;
use serde_json::json;
use tempfile::tempdir;

const MBID: &str = "11111111-2222-3333-4444-555555555555";
const DISC_ID: &str = "disc-id";

fn prepare_flac(src: &Path, dst: &Path, track_number: u32, with_album_id: bool) {
    std::fs::copy(src, dst).unwrap();
    let mut file = File::open(dst).unwrap();
    let mut flac =
        FlacFile::read_from(&mut BufReader::new(&mut file), ParseOptions::new()).unwrap();
    let mut comments = flac.vorbis_comments().cloned().unwrap_or_default();
    if with_album_id {
        comments.insert("MUSICBRAINZ_ALBUMID".into(), MBID.into());
        comments.insert("MUSICBRAINZ_DISCID".into(), DISC_ID.into());
    }
    comments.insert("TRACKNUMBER".into(), track_number.to_string());
    comments.insert("REPLAYGAIN_TRACK_GAIN".into(), "-3.00 dB".into());
    flac.set_vorbis_comments(comments);
    flac.save_to_path(dst, WriteOptions::default()).unwrap();
}

fn read_tags(path: &Path) -> VorbisComments {
    let mut file = File::open(path).unwrap();
    let flac = FlacFile::read_from(&mut BufReader::new(&mut file), ParseOptions::new()).unwrap();
    flac.vorbis_comments().cloned().unwrap_or_default()
}

fn mb_json() -> serde_json::Value {
    let credit = json!([{
        "name": "Test Artist",
        "joinphrase": "",
        "artist": {
            "id": "artist-mbid",
            "name": "Test Artist",
            "sort-name": "Artist, Test"
        }
    }]);
    json!({
        "id": MBID,
        "title": "Test Album",
        "status": "Official",
        "date": "2020-01-02",
        "country": "US",
        "barcode": "123456",
        "asin": "B000TEST",
        "artist-credit": credit,
        "release-group": {
            "id": "group-mbid",
            "title": "Test Album",
            "primary-type": "Album",
            "secondary-types": [],
            "first-release-date": "2019-12-01"
        },
        "label-info": [{
            "catalog-number": "CAT-1",
            "label": {"name": "Test Label"}
        }],
        "media": [{
            "position": 1,
            "format": "CD",
            "track-count": 2,
            "discs": [{"id": DISC_ID}],
            "tracks": [
                {
                    "id": "track-1-mbid",
                    "number": "1",
                    "title": "First",
                    "position": 1,
                    "length": 1000,
                    "artist-credit": credit,
                    "recording": {
                        "id": "rec-1-mbid",
                        "title": "First",
                        "length": 1000,
                        "isrcs": ["ISRC-ONE", "ISRC-TWO"],
                        "artist-credit": credit,
                        "relations": []
                    }
                },
                {
                    "id": "track-2-mbid",
                    "number": "2",
                    "title": "Second",
                    "position": 2,
                    "length": 1000,
                    "artist-credit": credit,
                    "recording": {
                        "id": "rec-2-mbid",
                        "title": "Second",
                        "length": 1000,
                        "isrcs": ["ISRC-THREE"],
                        "artist-credit": credit,
                        "relations": []
                    }
                }
            ]
        }],
        "text-representation": {"language": "eng", "script": "Latn"}
    })
}

struct TestServer {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let thread = thread::spawn(move || {
            while !stop_thread.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => handle(stream, port),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn mb_base(&self) -> String {
        format!("http://127.0.0.1:{}/ws/2", self.port)
    }

    fn caa_base(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn handle(mut stream: TcpStream, port: u16) {
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) => return,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
            Err(_) => return,
        }
        if request.len() > 64 * 1024 {
            return;
        }
    }
    let text = String::from_utf8_lossy(&request);
    let path = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let path = path.split('?').next().unwrap_or(path);

    if path.starts_with("/ws/2/release/") {
        respond(
            &mut stream,
            200,
            "application/json",
            mb_json().to_string().as_bytes(),
        );
    } else if path == format!("/release/{MBID}/front") {
        respond(
            &mut stream,
            200,
            "image/jpeg",
            include_bytes!("fixtures/front.jpg"),
        );
    } else if path == format!("/release/{MBID}") {
        let manifest = json!({
            "images": [
                {
                    "types": ["Back"],
                    "image": format!("http://127.0.0.1:{port}/images/back.jpg")
                },
                {
                    "types": ["Booklet"],
                    "image": format!("http://127.0.0.1:{port}/images/booklet.jpg")
                },
                {
                    "types": ["Booklet"],
                    "image": format!("http://127.0.0.1:{port}/images/booklet.pdf")
                }
            ]
        });
        respond(
            &mut stream,
            200,
            "application/json",
            manifest.to_string().as_bytes(),
        );
    } else if path == "/images/back.jpg" {
        respond(
            &mut stream,
            200,
            "image/jpeg",
            include_bytes!("fixtures/back.jpg"),
        );
    } else if path == "/images/booklet.jpg" {
        respond(
            &mut stream,
            200,
            "image/jpeg",
            include_bytes!("fixtures/booklet.jpg"),
        );
    } else if path == "/images/booklet.pdf" {
        respond(&mut stream, 200, "application/pdf", b"%PDF-1.4\n%%EOF\n");
    } else {
        respond(&mut stream, 404, "text/plain", b"not found");
    }
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let reason = if status == 200 { "OK" } else { "Not Found" };
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(headers.as_bytes());
    let _ = stream.write_all(body);
}

fn run_binary(album: &Path, server: &TestServer, extra: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_complete-album"))
        .arg(album)
        .arg("--mb-base")
        .arg(server.mb_base())
        .arg("--caa-base")
        .arg(server.caa_base())
        .arg("--user-agent")
        .arg("complete-album-test/0.1 (test@example.com)")
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn writes_picard_tags_and_artwork() {
    let server = TestServer::start();
    let temp = tempdir().unwrap();
    let album = temp.path();
    prepare_flac(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tiny.flac")
            .as_path(),
        &album.join("01.flac"),
        1,
        true,
    );
    prepare_flac(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tiny.flac")
            .as_path(),
        &album.join("02.flac"),
        2,
        true,
    );

    let output = run_binary(album, &server, &[]);
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tags written: 2 file(s)"), "{stdout}");

    let first = read_tags(&album.join("01.flac"));
    let second = read_tags(&album.join("02.flac"));
    assert_eq!(first.get("MUSICBRAINZ_ALBUMID"), Some(MBID));
    assert_eq!(first.get("MUSICBRAINZ_RELEASEGROUPID"), Some("group-mbid"));
    assert_eq!(first.get("MUSICBRAINZ_TRACKID"), Some("track-1-mbid"));
    assert_eq!(first.get("MUSICBRAINZ_RECORDINGID"), Some("rec-1-mbid"));
    assert_eq!(first.get("MUSICBRAINZ_ARTISTID"), Some("artist-mbid"));
    assert_eq!(first.get("MUSICBRAINZ_DISCID"), Some(DISC_ID));
    assert_eq!(first.get("REPLAYGAIN_TRACK_GAIN"), Some("-3.00 dB"));
    assert_eq!(first.get("ALBUMARTIST"), Some("Test Artist"));
    assert_eq!(first.get("ALBUMARTISTSORT"), Some("Artist, Test"));
    assert_eq!(first.get("TITLE"), Some("First"));
    assert_eq!(first.get("ORIGINALDATE"), Some("2019-12-01"));
    assert_eq!(first.get("ORIGINALYEAR"), Some("2019"));
    assert_eq!(first.get("RELEASETYPE"), Some("album"));
    assert_eq!(first.get("DISCTOTAL"), Some("1"));
    assert_eq!(
        first.get_all("ISRC").collect::<Vec<_>>(),
        vec!["ISRC-ONE", "ISRC-TWO"]
    );
    assert_eq!(second.get("MUSICBRAINZ_TRACKID"), Some("track-2-mbid"));
    assert_eq!(second.get("TITLE"), Some("Second"));

    assert!(album.join("cover.jpg").is_file());
    assert!(album.join("back.jpg").is_file());
    assert!(album.join("booklet-01.jpg").is_file());
    assert!(album.join("booklet-02.pdf").is_file());

    let mut file = File::open(album.join("01.flac")).unwrap();
    let flac = FlacFile::read_from(&mut BufReader::new(&mut file), ParseOptions::new()).unwrap();
    assert!(
        flac.pictures()
            .iter()
            .any(|(picture, _)| picture.pic_type() == PictureType::CoverFront)
    );
}

#[test]
fn dry_run_writes_nothing() {
    let server = TestServer::start();
    let temp = tempdir().unwrap();
    let album = temp.path();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny.flac");
    prepare_flac(&source, &album.join("01.flac"), 1, true);
    let before = std::fs::read(album.join("01.flac")).unwrap();

    let output = run_binary(album, &server, &["--dry-run"]);
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read(album.join("01.flac")).unwrap(), before);
    assert!(!album.join("cover.jpg").exists());
    assert!(!album.join("back.jpg").exists());
    assert!(!album.join("booklet-01.jpg").exists());
}
