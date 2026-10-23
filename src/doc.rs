//! The open trip: the trip itself, its file, and saving.
//!
//! Changes are saved by themselves a moment after the last edit. Every few
//! seconds the file is checked for changes saved elsewhere (the other
//! person, through Google Drive): with nothing unsaved here they're taken
//! straight in; with unsaved changes here, the person chooses whose to keep.

use std::time::{Duration, Instant};

use waypoint_core::file::{self, Lock, TripFile};
use waypoint_core::{Item, Trip};

/// How long after the last edit to save.
const SAVE_AFTER: Duration = Duration::from_millis(1500);
/// How long to wait before trying again after a failed save.
const RETRY_AFTER: Duration = Duration::from_secs(10);
/// How often to look for changes saved elsewhere.
const CHECK_EVERY: Duration = Duration::from_secs(3);

pub enum SaveState {
    Saved(Instant),
    Unsaved,
    Failed(String, Instant),
}

pub struct Doc {
    pub trip: Trip,
    /// None for the sample trip, which isn't saved.
    pub file: Option<TripFile>,
    pub state: SaveState,
    last_edit: Instant,
    last_check: Instant,
    /// Someone else saved while there were unsaved changes here.
    pub conflict: bool,
    /// Someone else had the trip open when it was opened here.
    pub elsewhere: Option<Lock>,
    /// The last item deleted, with where it was, for Undo.
    pub deleted: Option<(Item, usize, Instant)>,
    /// A short message, such as "Updated with changes from elsewhere".
    pub toast: Option<(String, Instant)>,
    pub me: String,
    /// The trip was replaced from disk this frame, so it isn't an edit.
    pub reloaded: bool,
}

pub fn now_secs() -> i64 {
    jiff::Timestamp::now().as_second()
}

impl Doc {
    pub fn new(trip: Trip, file: Option<TripFile>) -> Doc {
        let me = file::who_am_i();
        let elsewhere = file
            .as_ref()
            .and_then(|f| f.open_elsewhere(&me, now_secs()));
        if let Some(f) = &file {
            f.take_lock(&me, now_secs());
        }
        Doc {
            trip,
            file,
            state: SaveState::Saved(Instant::now()),
            last_edit: Instant::now(),
            last_check: Instant::now(),
            conflict: false,
            elsewhere,
            deleted: None,
            toast: None,
            me,
            reloaded: false,
        }
    }

    pub fn is_sample(&self) -> bool {
        self.file.is_none()
    }

    /// Note that the trip was changed this frame. Items that changed get
    /// stamped with when and by whom.
    pub fn edited(&mut self, before: &Trip) {
        let now = jiff::Timestamp::now();
        for item in &mut self.trip.items {
            let old = before.items.iter().find(|i| i.id == item.id);
            if old != Some(item) {
                item.updated = Some(now);
                item.updated_by = self.me.clone();
            }
        }
        self.last_edit = Instant::now();
        if self.file.is_some() {
            self.state = SaveState::Unsaved;
        }
    }

    pub fn toast(&mut self, s: impl Into<String>) {
        self.toast = Some((s.into(), Instant::now()));
    }

    /// Save if it's time, and look for changes from elsewhere. Call every
    /// frame. Returns how soon it wants to be called again.
    pub fn tick(&mut self, phone_page: bool) -> Option<Duration> {
        self.file.as_ref()?;
        if self.conflict {
            return None;
        }
        let due = match &self.state {
            SaveState::Unsaved => Some(self.last_edit + SAVE_AFTER),
            SaveState::Failed(_, at) => Some(*at + RETRY_AFTER),
            SaveState::Saved(_) => None,
        };
        if let Some(due) = due {
            if Instant::now() >= due {
                self.save(phone_page);
            } else {
                return Some(due - Instant::now());
            }
        }
        if self.last_check.elapsed() >= CHECK_EVERY {
            self.last_check = Instant::now();
            let file = self.file.as_mut()?;
            if file.changed_on_disk() {
                if matches!(self.state, SaveState::Saved(_)) {
                    match file.reload() {
                        Ok(t) => {
                            self.trip = t;
                            self.toast("Updated with changes saved elsewhere");
                        }
                        Err(e) => self.state = SaveState::Failed(e.to_string(), Instant::now()),
                    }
                } else {
                    self.conflict = true;
                }
            }
        }
        Some(CHECK_EVERY)
    }

    /// Save now (if there's a file).
    pub fn save(&mut self, phone_page: bool) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        match file.save(&self.trip) {
            Ok(()) => {
                self.state = SaveState::Saved(Instant::now());
                file.take_lock(&self.me, now_secs());
                if phone_page {
                    self.write_phone_page();
                }
            }
            Err(file::Error::Changed) => self.conflict = true,
            Err(e) => self.state = SaveState::Failed(e.to_string(), Instant::now()),
        }
    }

    /// Write the phone itinerary beside the trip. Returns where.
    pub fn write_phone_page(&mut self) -> Option<std::path::PathBuf> {
        let path = self.file.as_ref()?.html_path();
        let page = waypoint_core::html::render(&self.trip, jiff::Zoned::now());
        match file::write_atomic(&path, page.as_bytes()) {
            Ok(()) => Some(path),
            Err(e) => {
                self.toast(format!("Couldn't write the phone itinerary: {e}"));
                None
            }
        }
    }

    /// Resolve a conflict by keeping the other person's version, with this
    /// computer's version saved beside it as a copy.
    pub fn take_theirs(&mut self) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let copy = file.conflict_copy_path(&self.me);
        let mut mine = self.trip.clone();
        mine.name = format!("{} (copy from {})", mine.name, self.me);
        let copied = TripFile::create(&copy, &mine);
        match file.reload() {
            Ok(t) => {
                self.trip = t;
                self.reloaded = true;
                self.conflict = false;
                self.state = SaveState::Saved(Instant::now());
                match copied {
                    Ok(_) => {
                        let name = copy
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("a copy");
                        self.toast(format!("Loaded their changes. Yours are in \"{name}\"."))
                    }
                    Err(e) => self.toast(format!(
                        "Loaded their changes, but couldn't keep yours as a copy: {e}"
                    )),
                }
            }
            Err(e) => self.state = SaveState::Failed(e.to_string(), Instant::now()),
        }
    }

    /// Resolve a conflict by saving this computer's version over theirs.
    pub fn keep_mine(&mut self, phone_page: bool) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        match file.overwrite(&self.trip) {
            Ok(()) => {
                self.conflict = false;
                self.state = SaveState::Saved(Instant::now());
                if phone_page {
                    self.write_phone_page();
                }
            }
            Err(e) => self.state = SaveState::Failed(e.to_string(), Instant::now()),
        }
    }

    pub fn delete(&mut self, id: &str) {
        if let Some(idx) = self.trip.items.iter().position(|i| i.id == id) {
            let item = self.trip.items.remove(idx);
            self.deleted = Some((item, idx, Instant::now()));
        }
    }

    pub fn undo_delete(&mut self) -> Option<String> {
        let (item, idx, _) = self.deleted.take()?;
        let id = item.id.clone();
        let idx = idx.min(self.trip.items.len());
        self.trip.items.insert(idx, item);
        Some(id)
    }

    /// Save anything outstanding and let go of the lock. Called on the way
    /// out.
    pub fn close(&mut self, phone_page: bool) {
        if matches!(self.state, SaveState::Unsaved | SaveState::Failed(..)) && !self.conflict {
            self.save(phone_page);
        }
        if let Some(f) = &self.file {
            f.release_lock(&self.me);
        }
    }
}
