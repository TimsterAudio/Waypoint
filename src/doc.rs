//! The open trip: the trip itself, its file, and saving.
//!
//! Changes are saved by themselves a moment after the last edit. Every few
//! seconds the file is checked for changes saved elsewhere (the other
//! person, through Google Drive): with nothing unsaved here they're taken
//! straight in; with unsaved changes here, the person chooses whose to keep.
//!
//! Every change can be undone, back to when the trip was opened (or last
//! taken in from elsewhere). Typing in one box is one step.

use std::time::{Duration, Instant};

use eframe::egui;
use waypoint_core::file::{self, Lock, TripFile};
use waypoint_core::Trip;

/// How long after the last edit to save.
const SAVE_AFTER: Duration = Duration::from_millis(1500);
/// How long to wait before trying again after a failed save.
const RETRY_AFTER: Duration = Duration::from_secs(10);
/// How often to look for changes saved elsewhere.
const CHECK_EVERY: Duration = Duration::from_secs(3);
/// How many steps Undo goes back.
const UNDO_STEPS: usize = 300;
/// How long a message with Undo on it shows.
const NOTE_FOR: Duration = Duration::from_secs(12);

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
    /// The trip before each change, the latest last, for Undo.
    undo: Vec<Trip>,
    /// Changes undone, the latest last, for Redo.
    redo: Vec<Trip>,
    /// The box the last change was typed in, so typing in it again joins
    /// that step. None after anything else, so the next change is a step.
    step: Option<egui::Id>,
    /// A message with Undo on it ("Deleted …", "Reversed the trip"), and
    /// how many steps there were after it, so Undo shows only until the
    /// next change.
    pub note: Option<(String, Instant, usize)>,
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
            undo: Vec::new(),
            redo: Vec::new(),
            step: None,
            note: None,
            toast: None,
            me,
            reloaded: false,
        }
    }

    pub fn is_sample(&self) -> bool {
        self.file.is_none()
    }

    /// Note that the trip was changed, as a step of its own.
    pub fn edited(&mut self, before: &Trip) {
        self.edited_in(before, None);
    }

    /// Note that the trip was changed this frame, while typing in `focus`
    /// (if anything): more typing in the same box joins the same step.
    /// Items that changed get stamped with when and by whom.
    pub fn edited_in(&mut self, before: &Trip, focus: Option<egui::Id>) {
        if focus.is_none() || focus != self.step {
            self.undo.push(before.clone());
            if self.undo.len() > UNDO_STEPS {
                self.undo.remove(0);
            }
        }
        self.step = focus;
        self.redo.clear();
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

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Put the trip back as it was before the last change.
    pub fn undo(&mut self) -> bool {
        let Some(t) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.trip, t));
        self.restored();
        true
    }

    /// Make the last change undone again.
    pub fn redo(&mut self) -> bool {
        let Some(t) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.trip, t));
        self.restored();
        true
    }

    fn restored(&mut self) {
        self.step = None;
        self.note = None;
        self.last_edit = Instant::now();
        if self.file.is_some() {
            self.state = SaveState::Unsaved;
        }
    }

    /// Show a message with Undo on it for the change about to be noted
    /// (this frame's), such as "Reversed the trip".
    pub fn note_undo(&mut self, s: impl Into<String>) {
        self.step = None;
        self.note = Some((s.into(), Instant::now(), self.undo.len() + 1));
    }

    /// The message with Undo on it, while it's still the latest change.
    pub fn undo_note(&mut self) -> Option<String> {
        let (text, at, steps) = self.note.as_ref()?;
        // One step more while this frame's change is still to be noted.
        let latest = *steps == self.undo.len() || *steps == self.undo.len() + 1;
        if at.elapsed() < NOTE_FOR && latest {
            Some(text.clone())
        } else {
            self.note = None;
            None
        }
    }

    /// The trip was replaced by the version saved elsewhere: what's undone
    /// from here would undo their changes too, so the history starts again.
    fn forget_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.step = None;
        self.note = None;
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
                            self.forget_history();
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

    /// Write the page to share with family and friends beside the trip (or,
    /// for the sample, in the temporary folder). Returns where.
    pub fn write_share_page(&mut self) -> Option<std::path::PathBuf> {
        let path = match &self.file {
            Some(f) => f.share_path(),
            None => std::env::temp_dir().join("Waypoint sample (to share).html"),
        };
        let page = waypoint_core::html::render_shared(&self.trip, jiff::Zoned::now());
        match file::write_atomic(&path, page.as_bytes()) {
            Ok(()) => Some(path),
            Err(e) => {
                self.toast(format!("Couldn't write the itinerary to share: {e}"));
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
                self.forget_history();
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
            self.note_undo(format!("Deleted {}", item.display_title()));
        }
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
