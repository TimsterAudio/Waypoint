//! Trip files on disk, usually in a Google Drive folder.
//!
//! A trip is one `Name.trip.json` file. Saving writes a temporary file and
//! renames it over the old one, so a sync client never uploads half a file.
//! Before saving, the file on disk is checked against what was loaded: if
//! someone else saved in between, the save stops and says so rather than
//! quietly overwriting their changes.
//!
//! While a trip is open, a small `Name.trip.lock` beside it says who has it
//! open, so a second person gets a warning.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Trip, FORMAT};

pub const EXTENSION: &str = ".trip.json";
const LOCK_EXTENSION: &str = ".trip.lock";
/// A lock older than this is assumed to be left over from a crash.
const LOCK_STALE_SECS: i64 = 12 * 60 * 60;

#[derive(Debug)]
pub enum Error {
    Io(PathBuf, std::io::Error),
    Parse(PathBuf, serde_json::Error),
    /// The file is from a newer Waypoint.
    TooNew(u32),
    /// Someone else saved the file after it was opened here.
    Changed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(p, e) => write!(f, "Couldn't use {}: {e}", p.display()),
            Error::Parse(p, e) => write!(
                f,
                "{} isn't a trip file Waypoint can read: {e}",
                p.display()
            ),
            Error::TooNew(v) => write!(
                f,
                "This trip was saved by a newer Waypoint (format {v}). Update Waypoint to open it."
            ),
            Error::Changed => write!(f, "Someone else changed this trip since you opened it."),
        }
    }
}

impl std::error::Error for Error {}

/// A cheap fingerprint of a file's bytes (FNV-1a), to notice changes.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// An open trip file.
#[derive(Debug, Clone)]
pub struct TripFile {
    pub path: PathBuf,
    /// The fingerprint of the file as last read or written here.
    pub seen: u64,
}

impl TripFile {
    /// The file name for a trip called `name`: "Japan 2027.trip.json".
    pub fn file_name(name: &str) -> String {
        let clean: String = name
            .chars()
            .map(|c| {
                if "/\\:*?\"<>|".contains(c) || c.is_control() {
                    '-'
                } else {
                    c
                }
            })
            .collect();
        let clean = clean.trim().trim_matches('.');
        format!(
            "{}{EXTENSION}",
            if clean.is_empty() { "Trip" } else { clean }
        )
    }

    pub fn open(path: &Path) -> Result<(TripFile, Trip), Error> {
        let bytes = fs::read(path).map_err(|e| Error::Io(path.into(), e))?;
        let trip = parse(path, &bytes)?;
        Ok((
            TripFile {
                path: path.into(),
                seen: fingerprint(&bytes),
            },
            trip,
        ))
    }

    /// Create a new trip file. Fails if one is already there.
    pub fn create(path: &Path, trip: &Trip) -> Result<TripFile, Error> {
        if path.exists() {
            return Err(Error::Io(
                path.into(),
                std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "a file with that name is already there",
                ),
            ));
        }
        let mut f = TripFile {
            path: path.into(),
            seen: 0,
        };
        f.write(trip)?;
        Ok(f)
    }

    /// Has the file on disk changed since it was read or written here?
    /// A file that's gone, or can't be read for now, doesn't count.
    pub fn changed_on_disk(&self) -> bool {
        match fs::read(&self.path) {
            Ok(bytes) => fingerprint(&bytes) != self.seen,
            Err(_) => false,
        }
    }

    /// Read the file again, taking whatever is on disk now.
    pub fn reload(&mut self) -> Result<Trip, Error> {
        let (f, trip) = TripFile::open(&self.path)?;
        self.seen = f.seen;
        Ok(trip)
    }

    /// Save, unless someone else saved first.
    pub fn save(&mut self, trip: &Trip) -> Result<(), Error> {
        if self.changed_on_disk() {
            return Err(Error::Changed);
        }
        self.write(trip)
    }

    /// Save whatever is on disk.
    pub fn overwrite(&mut self, trip: &Trip) -> Result<(), Error> {
        self.write(trip)
    }

    fn write(&mut self, trip: &Trip) -> Result<(), Error> {
        let mut trip = trip.clone();
        trip.format = FORMAT;
        let mut bytes = serde_json::to_vec_pretty(&trip).expect("a trip always serialises");
        bytes.push(b'\n');
        write_atomic(&self.path, &bytes)?;
        self.seen = fingerprint(&bytes);
        Ok(())
    }

    /// Where a copy goes when two people's changes clash:
    /// "Japan 2027 (copy from laptop).trip.json".
    pub fn conflict_copy_path(&self, who: &str) -> PathBuf {
        let name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Trip.trip.json");
        let stem = name.strip_suffix(EXTENSION).unwrap_or(name);
        let dir = self.path.parent().unwrap_or(Path::new("."));
        let mut n = 1;
        loop {
            let suffix = if n == 1 {
                String::new()
            } else {
                format!(" {n}")
            };
            let p = dir.join(format!("{stem} (copy from {who}{suffix}){EXTENSION}"));
            if !p.exists() {
                return p;
            }
            n += 1;
        }
    }

    /// The phone itinerary beside the trip: "Japan 2027.html".
    pub fn html_path(&self) -> PathBuf {
        let name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Trip.trip.json");
        let stem = name.strip_suffix(EXTENSION).unwrap_or(name);
        self.path.with_file_name(format!("{stem}.html"))
    }

    /// The page to share beside the trip: "Japan 2027 (to share).html".
    pub fn share_path(&self) -> PathBuf {
        let html = self.html_path();
        let stem = html
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Trip")
            .to_string();
        html.with_file_name(format!("{stem} (to share).html"))
    }

    fn lock_path(&self) -> PathBuf {
        let name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Trip.trip.json");
        let stem = name.strip_suffix(EXTENSION).unwrap_or(name);
        self.path.with_file_name(format!("{stem}{LOCK_EXTENSION}"))
    }

    /// Who else has this trip open, if anyone (and it isn't stale).
    pub fn open_elsewhere(&self, me: &str, now_secs: i64) -> Option<Lock> {
        let bytes = fs::read(self.lock_path()).ok()?;
        let lock: Lock = serde_json::from_slice(&bytes).ok()?;
        (lock.who != me && now_secs - lock.since < LOCK_STALE_SECS).then_some(lock)
    }

    /// Say this trip is open here. Best effort: a lock that can't be written
    /// only means nobody gets warned.
    pub fn take_lock(&self, me: &str, now_secs: i64) {
        let lock = Lock {
            who: me.into(),
            since: now_secs,
        };
        let _ = write_atomic(
            &self.lock_path(),
            &serde_json::to_vec(&lock).unwrap_or_default(),
        );
    }

    /// Remove our lock, if it's still ours.
    pub fn release_lock(&self, me: &str) {
        let p = self.lock_path();
        if let Ok(bytes) = fs::read(&p) {
            if serde_json::from_slice::<Lock>(&bytes).is_ok_and(|l| l.who == me) {
                let _ = fs::remove_file(p);
            }
        }
    }
}

/// Who has a trip open, and since when (seconds since 1970).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lock {
    pub who: String,
    pub since: i64,
}

fn parse(path: &Path, bytes: &[u8]) -> Result<Trip, Error> {
    let trip: Trip = serde_json::from_slice(bytes).map_err(|e| Error::Parse(path.into(), e))?;
    if trip.format > FORMAT {
        return Err(Error::TooNew(trip.format));
    }
    Ok(trip)
}

/// Write to a temporary file beside `path`, then rename it into place.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = dir.join(format!(".{name}.saving"));
    let io = |e| Error::Io(path.into(), e);
    {
        let mut f = fs::File::create(&tmp).map_err(io)?;
        f.write_all(bytes).map_err(io)?;
        f.sync_all().map_err(io)?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Error::Io(path.into(), e)
    })
}

/// "tim on laptop": who is saving, for locks and conflict copies.
pub fn who_am_i() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .or_else(|| fs::read_to_string("/etc/hostname").ok())
        .map(|h| h.trim().to_string())
        .unwrap_or_default();
    match (user.is_empty(), host.is_empty()) {
        (false, false) => format!("{user} on {host}"),
        (false, true) => user,
        (true, false) => host,
        (true, true) => "another computer".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("waypoint-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(TripFile::file_name("Japan 2027"), "Japan 2027.trip.json");
        assert_eq!(TripFile::file_name("NZ: N/S"), "NZ- N-S.trip.json");
        assert_eq!(TripFile::file_name("  "), "Trip.trip.json");
    }

    #[test]
    fn save_stops_when_someone_else_saved() {
        let d = dir("conflict");
        let p = d.join("T.trip.json");
        let mut mine = TripFile::create(&p, &Trip::new("T")).unwrap();
        let (mut theirs, mut t2) = TripFile::open(&p).unwrap();
        t2.notes = "theirs".into();
        theirs.save(&t2).unwrap();

        let mut t1 = Trip::new("T");
        t1.notes = "mine".into();
        assert!(mine.changed_on_disk());
        assert!(matches!(mine.save(&t1), Err(Error::Changed)));
        assert_eq!(mine.reload().unwrap().notes, "theirs");
        // After reloading, saving works again.
        mine.save(&t1).unwrap();
        assert!(!mine.changed_on_disk());
        assert!(mine
            .conflict_copy_path("tim on laptop")
            .ends_with("T (copy from tim on laptop).trip.json"));
        assert!(mine.html_path().ends_with("T.html"));
        assert!(mine.share_path().ends_with("T (to share).html"));
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn locks_warn_others_but_not_me() {
        let d = dir("lock");
        let f = TripFile::create(&d.join("T.trip.json"), &Trip::new("T")).unwrap();
        f.take_lock("me", 1000);
        assert!(f.open_elsewhere("me", 1001).is_none());
        assert_eq!(f.open_elsewhere("you", 1001).unwrap().who, "me");
        assert!(f
            .open_elsewhere("you", 1000 + LOCK_STALE_SECS + 1)
            .is_none());
        f.release_lock("you");
        assert!(f.open_elsewhere("you", 1001).is_some());
        f.release_lock("me");
        assert!(f.open_elsewhere("you", 1001).is_none());
        let _ = fs::remove_dir_all(d);
    }

    #[test]
    fn refuses_a_newer_format() {
        let d = dir("newer");
        let p = d.join("T.trip.json");
        fs::write(&p, br#"{"format": 99, "name": "T"}"#).unwrap();
        assert!(matches!(TripFile::open(&p), Err(Error::TooNew(99))));
        let _ = fs::remove_dir_all(d);
    }
}
