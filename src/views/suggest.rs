//! Places as you type, in the item editor: after a short pause in typing a
//! name, ask the place search and list what it finds under the box. Picking
//! one fills in the name and address (and the time zone, when the item has
//! none). Only used when switched on in Settings.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui::{self, vec2, CornerRadius, CursorIcon, Sense, Ui};
use waypoint_core::lookup::{self, Found};

use crate::theme;
use crate::ui;

/// How long typing must pause before asking.
const PAUSE: Duration = Duration::from_millis(450);
/// The shortest text worth asking about.
const MIN_LEN: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Field {
    #[default]
    Title,
    Place,
}

type Answer = (String, Result<Vec<Found>, String>);

#[derive(Default)]
pub struct Suggest {
    /// The item and box being typed in.
    item: String,
    field: Field,
    query: String,
    typed_at: Option<Instant>,
    /// The text last sent, and the answer on its way.
    asked: Option<String>,
    rx: Option<mpsc::Receiver<Answer>>,
    results: Vec<Found>,
    error: Option<String>,
}

impl Suggest {
    /// Note what's in a box after drawing it.
    pub fn typed(&mut self, item: &str, field: Field, text: &str, changed: bool) {
        if !changed {
            return;
        }
        if self.item != item || self.field != field {
            *self = Suggest::default();
            self.item = item.to_string();
            self.field = field;
        }
        self.query = text.trim().to_string();
        self.typed_at = Some(Instant::now());
        self.results.clear();
        self.error = None;
    }

    /// Ask when typing has paused, and take in answers. Call every frame.
    pub fn tick(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.rx {
            if let Ok((asked, answer)) = rx.try_recv() {
                self.rx = None;
                if asked == self.query {
                    match answer {
                        Ok(found) => self.results = found,
                        Err(e) => self.error = Some(e),
                    }
                }
            }
        }
        let Some(at) = self.typed_at else { return };
        // Already asked about this text (an answer to older text that's
        // still on its way is dropped when the new question goes).
        if self.query.chars().count() < MIN_LEN
            || self.asked.as_deref() == Some(self.query.as_str())
        {
            return;
        }
        let wait = PAUSE.saturating_sub(at.elapsed());
        if !wait.is_zero() {
            ctx.request_repaint_after(wait);
            return;
        }
        let q = self.query.clone();
        self.asked = Some(q.clone());
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let answer = lookup::search(&q);
            let _ = tx.send((q, answer));
            ctx.request_repaint();
        });
        self.rx = Some(rx);
    }

    /// Stop suggesting (a place was picked, or the box was left empty).
    pub fn clear(&mut self) {
        *self = Suggest::default();
    }

    /// The list under a box, if it's the one being typed in. Returns the
    /// place picked.
    pub fn list(&mut self, ui: &mut Ui, item: &str, field: Field) -> Option<Found> {
        if self.item != item || self.field != field || self.query.is_empty() {
            return None;
        }
        let p = ui::pal(ui);
        if let Some(e) = &self.error {
            ui::small(ui, format!("Couldn't search for places: {e}"), p.ink2);
            return None;
        }
        if self.rx.is_some() && self.results.is_empty() {
            ui::small(ui, "Looking for places…", p.ink2);
            return None;
        }
        if self.results.is_empty() {
            if self.asked.as_deref() == Some(self.query.as_str()) && self.rx.is_none() {
                ui::small(ui, "No places found", p.ink2);
            }
            return None;
        }
        let mut picked = None;
        egui::Frame::new()
            .fill(p.paper)
            .stroke(egui::Stroke::new(1.0, p.grid))
            .corner_radius(CornerRadius::same(theme::RADIUS))
            .inner_margin(egui::Margin::same(4))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for f in &self.results {
                    let w = ui.available_width();
                    let (rect, resp) = ui.allocate_exact_size(vec2(w, 42.0), Sense::click());
                    if resp.hovered() {
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(theme::RADIUS), p.faint);
                    }
                    let clip = ui.painter().with_clip_rect(rect.shrink(2.0));
                    let mut name = f.name.clone();
                    if !f.what.is_empty() {
                        name.push_str(&format!("  ·  {}", f.what));
                    }
                    clip.text(
                        rect.left_top() + vec2(8.0, 12.0),
                        egui::Align2::LEFT_CENTER,
                        name,
                        theme::semi(14.0),
                        p.ink,
                    );
                    clip.text(
                        rect.left_top() + vec2(8.0, 30.0),
                        egui::Align2::LEFT_CENTER,
                        &f.address,
                        theme::body(13.0),
                        p.ink2,
                    );
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        picked = Some(f.clone());
                    }
                }
                ui.add_space(2.0);
                ui::small(ui, "Places from OpenStreetMap", ui::pal(ui).ink2);
            });
        if picked.is_some() {
            self.clear();
        }
        picked
    }
}
