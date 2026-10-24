//! Shared pieces of the interface: panels, chips, buttons, legend keys,
//! headings, columns and tables.

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Frame, Layout,
    Margin, Rect, Response, Sense, Stroke, StrokeKind, Ui,
};

use crate::theme::{self, alpha, mix, Palette, RADIUS};

pub fn pal(ui: &Ui) -> Palette {
    Palette::of(ui.ctx())
}

/// A sheet of paper on the desk: the frame round each chart and table.
pub fn panel<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui);
    Frame::new()
        .fill(p.sheet)
        .stroke(Stroke::new(1.0, p.grid))
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(Margin {
            left: 24,
            right: 24,
            top: 18,
            bottom: 22,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 12.0;
            add(ui)
        })
        .inner
}

pub fn h1(ui: &mut Ui, text: &str) {
    let p = pal(ui);
    ui.label(
        egui::RichText::new(text)
            .font(theme::display(32.0))
            .color(p.ink),
    );
}

pub fn h2(ui: &mut Ui, text: &str) {
    let p = pal(ui);
    ui.label(
        egui::RichText::new(text)
            .font(theme::display(20.0))
            .color(p.ink),
    );
}

pub fn muted(ui: &mut Ui, text: impl Into<String>) -> Response {
    let p = pal(ui);
    ui.label(
        egui::RichText::new(text.into())
            .font(theme::body(14.0))
            .color(p.ink2),
    )
}

pub fn small(ui: &mut Ui, text: impl Into<String>, color: Color32) -> Response {
    ui.label(
        egui::RichText::new(text.into())
            .font(theme::body(13.0))
            .color(color),
    )
}

pub fn text(ui: &mut Ui, text: impl Into<String>, font: FontId, color: Color32) -> Response {
    ui.label(egui::RichText::new(text.into()).font(font).color(color))
}

/// Wrapped text no wider than `max`.
pub fn para(ui: &mut Ui, s: impl Into<String>, font: FontId, color: Color32, max: f32) {
    let w = ui.available_width().min(max);
    ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_max_width(w);
        ui.add(egui::Label::new(egui::RichText::new(s.into()).font(font).color(color)).wrap());
    });
}

/// A pill-shaped choice, dark when chosen.
pub fn chip(ui: &mut Ui, label: &str, on: bool) -> Response {
    let p = pal(ui);
    let font = theme::medium(14.0);
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, p.ink);
    let size = vec2(galley.size().x + 26.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let (fill, border, color) = if on {
        (p.ink, p.ink, p.paper)
    } else if hovered {
        (Color32::TRANSPARENT, p.ink2, p.ink)
    } else {
        (Color32::TRANSPARENT, p.grid, p.ink2)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(16),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme::medium(14.0),
        color,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A row of chips; returns the index clicked.
pub fn chips(ui: &mut Ui, labels: &[&str], selected: Option<usize>) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for (i, l) in labels.iter().enumerate() {
            if chip(ui, l, selected == Some(i)).clicked() {
                clicked = Some(i);
            }
        }
    });
    clicked
}

#[derive(Clone, Copy, PartialEq)]
pub enum Btn {
    Primary,
    Secondary,
    Danger,
}

pub fn button(ui: &mut Ui, label: &str, kind: Btn) -> Response {
    let p = pal(ui);
    let font = theme::semi(14.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), p.ink);
    let size = vec2(galley.size().x + 32.0, 36.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let h = response.hovered();
    let (fill, border, color) = match kind {
        Btn::Primary if h => (
            p.trace,
            p.trace,
            if p.dark { p.paper } else { Color32::WHITE },
        ),
        Btn::Primary => (p.ink, p.ink, p.paper),
        Btn::Secondary => (Color32::TRANSPARENT, if h { p.ink2 } else { p.grid }, p.ink),
        Btn::Danger => (
            Color32::TRANSPARENT,
            if h { p.heat } else { p.grid },
            p.heat,
        ),
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(RADIUS),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, font, color);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// `n` equal columns `gap` apart. With `dividers`, a faint rule separates
/// them. `f` fills column `i`.
pub fn columns(ui: &mut Ui, n: usize, gap: f32, dividers: bool, mut f: impl FnMut(usize, &mut Ui)) {
    let n = n.max(1);
    let total = ui.available_width();
    let w = ((total - gap * (n - 1) as f32) / n as f32).max(10.0);
    let p = pal(ui);
    let r = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        let mut rects = Vec::new();
        for i in 0..n {
            let r = ui.vertical(|ui| {
                ui.set_width(w);
                f(i, ui);
            });
            rects.push(r.response.rect);
        }
        rects
    });
    if dividers {
        let rects = r.inner;
        let top = r.response.rect.top();
        let bottom = r.response.rect.bottom();
        for rr in rects.iter().skip(1) {
            let x = rr.left() - gap / 2.0;
            ui.painter()
                .line_segment([pos2(x, top), pos2(x, bottom)], Stroke::new(1.0, p.faint));
        }
    }
}

/// Two columns sharing the width `left : right`.
pub fn split<A, B>(
    ui: &mut Ui,
    ratio: f32,
    gap: f32,
    left: impl FnOnce(&mut Ui) -> A,
    right: impl FnOnce(&mut Ui) -> B,
) {
    let total = ui.available_width();
    let lw = ((total - gap) * ratio).floor();
    let rw = total - gap - lw;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.vertical(|ui| {
            ui.set_width(lw);
            left(ui);
        });
        ui.vertical(|ui| {
            ui.set_width(rw);
            right(ui);
        });
    });
}

/// [`split`], for two halves that both need the same state.
pub fn split_with<S>(
    ui: &mut Ui,
    ratio: f32,
    gap: f32,
    state: &mut S,
    left: impl FnOnce(&mut Ui, &mut S),
    right: impl FnOnce(&mut Ui, &mut S),
) {
    let total = ui.available_width();
    let lw = ((total - gap) * ratio).floor();
    let rw = total - gap - lw;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.vertical(|ui| {
            ui.set_width(lw);
            left(ui, state);
        });
        ui.vertical(|ui| {
            ui.set_width(rw);
            right(ui, state);
        });
    });
}

/// A full-width rule.
pub fn rule(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, color),
    );
}

/// A centred message for a screen with nothing to show yet. Returns true if
/// its button was pressed.
pub fn empty(ui: &mut Ui, title: &str, body: &str, button_label: Option<&str>) -> bool {
    let buttons: Vec<(&str, Btn)> = button_label
        .map(|b| (b, Btn::Primary))
        .into_iter()
        .collect();
    empty_with(ui, title, body, &buttons).is_some()
}

/// [`empty`] with a row of buttons. Returns the index of the one pressed.
pub fn empty_with(ui: &mut Ui, title: &str, body: &str, buttons: &[(&str, Btn)]) -> Option<usize> {
    let p = pal(ui);
    let mut clicked = None;
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        ui.label(
            egui::RichText::new(title)
                .font(theme::display(22.0))
                .color(p.ink),
        );
        let w = 440.0f32.min(ui.available_width());
        ui.allocate_ui(vec2(w, 0.0), |ui| {
            ui.set_max_width(w);
            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(body)
                            .font(theme::body(15.0))
                            .color(p.ink2),
                    )
                    .wrap(),
                );
            });
        });
        if !buttons.is_empty() {
            let gap = 10.0;
            let widths: f32 = buttons
                .iter()
                .map(|(b, _)| {
                    ui.painter()
                        .layout_no_wrap(b.to_string(), theme::semi(14.0), p.ink)
                        .size()
                        .x
                        + 32.0
                })
                .sum::<f32>()
                + gap * (buttons.len() - 1) as f32;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.add_space(((ui.available_width() - widths) / 2.0).max(0.0));
                for (i, (b, kind)) in buttons.iter().enumerate() {
                    if button(ui, b, *kind).clicked() {
                        clicked = Some(i);
                    }
                }
            });
        }
    });
    ui.add_space(40.0);
    clicked
}

// ---------------------------------------------------------------- tables

pub struct Col {
    pub title: String,
    /// Fixed width, or 0 to share what's left.
    pub width: f32,
    pub right: bool,
}

pub fn col(title: impl Into<String>, width: f32, right: bool) -> Col {
    Col {
        title: title.into(),
        width,
        right,
    }
}

#[derive(Default, Clone)]
pub struct Cell {
    pub text: String,
    pub color: Option<Color32>,
    pub strong: bool,
    /// Smaller grey text after the main text ("partial").
    pub note: Option<String>,
    pub tooltip: Option<String>,
}

pub fn cell(text: impl Into<String>) -> Cell {
    Cell {
        text: text.into(),
        ..Default::default()
    }
}

impl Cell {
    pub fn color(mut self, c: Color32) -> Self {
        self.color = Some(c);
        self
    }
    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }
    pub fn tooltip(mut self, t: impl Into<String>) -> Self {
        self.tooltip = Some(t.into());
        self
    }
}

/// A data table: a ruled header and faintly ruled rows. Rows light up under
/// the pointer; returns the row clicked. `selected` is drawn highlighted.
pub fn table(
    ui: &mut Ui,
    cols: &[Col],
    rows: &[Vec<Cell>],
    font_size: f32,
    selected: Option<usize>,
) -> Option<usize> {
    let p = pal(ui);
    let total = ui.available_width();
    // Fixed columns shrink in proportion when the table is narrow, leaving
    // each flexible column at least 44 px.
    let fixed: f32 = cols.iter().map(|c| c.width).sum();
    let n_flex = cols.iter().filter(|c| c.width == 0.0).count();
    let room = (total - 44.0 * n_flex as f32).max(0.0);
    let scale = if fixed > room && fixed > 0.0 {
        room / fixed
    } else {
        1.0
    };
    let flex_w = if n_flex > 0 {
        ((total - fixed * scale) / n_flex as f32).max(44.0)
    } else {
        0.0
    };
    let widths: Vec<f32> = cols
        .iter()
        .map(|c| {
            if c.width == 0.0 {
                flex_w
            } else {
                c.width * scale
            }
        })
        .collect();
    let row_h = font_size * 2.15;
    let head_h = 30.0;
    let height = head_h + row_h * rows.len() as f32;
    let (rect, response) = ui.allocate_exact_size(vec2(total, height), Sense::click());
    let painter = ui.painter_at(rect.expand(1.0));
    let row_at = |y: f32| -> Option<usize> {
        let k = ((y - rect.top() - head_h) / row_h).floor();
        (k >= 0.0 && (k as usize) < rows.len()).then_some(k as usize)
    };
    let hovered_row = response.hover_pos().and_then(|h| row_at(h.y));
    for (k, fill) in [
        (hovered_row, mix(p.sheet, p.faint, 0.7)),
        (selected, alpha(p.trace, 0.12)),
    ] {
        if let Some(k) = k {
            painter.rect_filled(
                Rect::from_min_size(
                    pos2(rect.left(), rect.top() + head_h + k as f32 * row_h),
                    vec2(total, row_h),
                ),
                0,
                fill,
            );
        }
    }
    let pad = 8.0;
    let mut x = rect.left();
    for (c, w) in cols.iter().zip(&widths) {
        let (pos, align) = if c.right {
            (
                pos2(x + w - pad, rect.top() + head_h / 2.0),
                Align2::RIGHT_CENTER,
            )
        } else {
            (
                pos2(x + pad, rect.top() + head_h / 2.0),
                Align2::LEFT_CENTER,
            )
        };
        painter.text(pos, align, &c.title, theme::semi(13.0), p.ink2);
        x += w;
    }
    painter.line_segment(
        [
            pos2(rect.left(), rect.top() + head_h),
            pos2(rect.right(), rect.top() + head_h),
        ],
        Stroke::new(1.0, p.grid),
    );
    let hover = response.hover_pos();
    let mut tip: Option<String> = None;
    for (ri, row) in rows.iter().enumerate() {
        let top = rect.top() + head_h + ri as f32 * row_h;
        let mid = top + row_h / 2.0;
        let mut x = rect.left();
        for ((c, w), cell) in cols.iter().zip(&widths).zip(row) {
            let font = if cell.strong {
                theme::semi(font_size)
            } else {
                theme::body(font_size)
            };
            let color = cell.color.unwrap_or(p.ink);
            let galley = painter.layout_no_wrap(cell.text.clone(), font, color);
            // Long text stops at its column's edge.
            let painter = painter.with_clip_rect(
                Rect::from_min_size(pos2(x + 2.0, top), vec2(w - 4.0, row_h))
                    .intersect(painter.clip_rect()),
            );
            let note = cell
                .note
                .as_ref()
                .map(|n| painter.layout_no_wrap(n.clone(), theme::body(12.0), p.ink2));
            let content_w = galley.size().x + note.as_ref().map_or(0.0, |n| n.size().x + 4.0);
            let left = if c.right {
                x + w - pad - content_w
            } else {
                x + pad
            };
            let gh = galley.size().y;
            painter.galley(pos2(left, mid - gh / 2.0), galley, color);
            if let Some(n) = note {
                let nh = n.size().y;
                painter.galley(
                    pos2(left + content_w - n.size().x, mid - nh / 2.0 + 1.0),
                    n,
                    p.ink2,
                );
            }
            if let (Some(t), Some(h)) = (&cell.tooltip, hover) {
                if Rect::from_min_size(pos2(x, top), vec2(*w, row_h)).contains(h) {
                    tip = Some(t.clone());
                }
            }
            x += w;
        }
        painter.line_segment(
            [
                pos2(rect.left(), top + row_h),
                pos2(rect.right(), top + row_h),
            ],
            Stroke::new(1.0, p.faint),
        );
    }
    let clicked = if response.clicked() {
        response.interact_pointer_pos().and_then(|q| row_at(q.y))
    } else {
        None
    };
    let response = if hovered_row.is_some() {
        response.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        response
    };
    if let Some(t) = tip {
        response.on_hover_text_at_pointer(t);
    }
    clicked
}

/// A short bar showing how far a value is along 0..=1.
pub fn meter(ui: &mut Ui, width: f32, frac: f32, color: Color32) {
    let p = pal(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
    let bar = Rect::from_center_size(rect.center(), vec2(width, 6.0));
    ui.painter().rect_filled(bar, 3, p.faint);
    let mut fill = bar;
    fill.set_width(width * frac.clamp(0.0, 1.0));
    ui.painter().rect_filled(fill, 3, color);
}

/// A faint banner, for notices like "You're looking at the demo station".
pub fn notice(ui: &mut Ui, s: &str) {
    let p = pal(ui);
    Frame::new()
        .fill(mix(p.faint, p.trace, 0.06))
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(s).font(theme::body(14.0)).color(p.ink));
        });
}

/// A labelled form field: bold label, the input, then optional help text.
pub fn field<R>(ui: &mut Ui, label: &str, help: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui);
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.label(
        egui::RichText::new(label)
            .font(theme::semi(14.0))
            .color(p.ink),
    );
    let r = add(ui);
    if let Some(h) = help {
        para(ui, h, theme::body(13.0), p.ink2, 560.0);
    }
    ui.add_space(6.0);
    ui.spacing_mut().item_spacing.y = 12.0;
    r
}

/// The single-line text input used throughout Settings.
pub fn text_input(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> Response {
    let p = pal(ui);
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(egui::RichText::new(hint).color(alpha(p.ink2, 0.8)))
            .font(theme::medium(14.0))
            .desired_width(width)
            .margin(Margin::symmetric(10, 8)),
    )
}

/// Reads a date typed day first: "14/5", "14/5/27", "14 May", "Sat 14 May
/// 2027", "2027-05-14". A missing year is taken from `like`.
pub fn parse_date(text: &str, like: jiff::civil::Date) -> Option<jiff::civil::Date> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let t = text.trim().to_lowercase();
    let parts: Vec<&str> = t
        .split(|c: char| c == '/' || c == '-' || c == '.' || c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .collect();
    // ISO: 2027-05-14.
    if parts.len() == 3 && parts[0].len() == 4 {
        let y = parts[0].parse().ok()?;
        return jiff::civil::Date::new(y, parts[1].parse().ok()?, parts[2].parse().ok()?).ok();
    }
    let mut day: Option<i8> = None;
    let mut month: Option<i8> = None;
    let mut year: Option<i16> = None;
    for p in parts {
        if let Ok(n) = p.parse::<i16>() {
            if day.is_none() {
                day = Some(i8::try_from(n).ok()?);
            } else if month.is_none() {
                month = Some(i8::try_from(n).ok()?);
            } else if year.is_none() {
                year = Some(match p.len() {
                    2 => 2000 + n,
                    4 => n,
                    _ => return None,
                });
            } else {
                return None;
            }
        } else if p.len() >= 3 {
            if let Some(m) = MONTHS.iter().position(|m| p.starts_with(m)) {
                month = Some(m as i8 + 1);
            }
            // Anything else ("sat", "saturday") is a weekday and ignored.
        } else {
            return None;
        }
    }
    jiff::civil::Date::new(year.unwrap_or(like.year()), month?, day?).ok()
}

/// A date: a box to type it in, day first, and a calendar button. One click
/// on a day in the calendar picks it and closes it. Returns true if changed.
pub fn date_input(ui: &mut Ui, id: &str, date: &mut jiff::civil::Date) -> bool {
    let p = pal(ui);
    let before = *date;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let shown = date.strftime("%a %-d %b %Y").to_string();
        let (r, typed) = typed_input(ui, egui::Id::new((id, "typed")), shown, "14/5", 128.0);
        if let Some(t) = typed {
            if let Some(d) = parse_date(&t, *date) {
                *date = d;
            }
        }
        r.on_hover_text("Type a date, day first: 14/5, 14 May or 14/5/27");

        let (rect, button) = ui.allocate_exact_size(vec2(30.0, 34.0), Sense::click());
        let hovered = button.hovered();
        if hovered {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(RADIUS), p.faint);
        }
        calendar_icon(
            ui.painter(),
            rect.center(),
            if hovered { p.ink } else { p.ink2 },
        );
        let button = button
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text("Pick from a calendar");
        let shown_month = egui::Id::new((id, "month"));
        if button.clicked() {
            ui.data_mut(|d| d.insert_temp(shown_month, (date.year(), date.month())));
        }
        let picked = egui::Popup::menu(&button)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                let (mut y, mut m) = ui
                    .data(|d| d.get_temp::<(i16, i8)>(shown_month))
                    .unwrap_or((date.year(), date.month()));
                let picked = month_grid(ui, &mut y, &mut m, *date);
                ui.data_mut(|d| d.insert_temp(shown_month, (y, m)));
                picked
            })
            .and_then(|r| r.inner);
        if let Some(d) = picked {
            *date = d;
            egui::Popup::close_id(ui.ctx(), egui::Popup::default_response_id(&button));
        }
    });
    *date != before
}

fn calendar_icon(painter: &egui::Painter, c: egui::Pos2, color: Color32) {
    let r = Rect::from_center_size(c + vec2(0.0, 1.0), vec2(14.0, 12.0));
    let stroke = Stroke::new(1.5, color);
    painter.rect_stroke(r, 2, stroke, StrokeKind::Middle);
    painter.line_segment(
        [
            pos2(r.left(), r.top() + 3.5),
            pos2(r.right(), r.top() + 3.5),
        ],
        stroke,
    );
    for x in [r.left() + 4.0, r.right() - 4.0] {
        painter.line_segment([pos2(x, r.top() - 2.5), pos2(x, r.top() + 1.0)], stroke);
    }
}

/// A month of days, Monday first, with arrows to move a month. Returns the
/// day clicked.
fn month_grid(
    ui: &mut Ui,
    year: &mut i16,
    month: &mut i8,
    selected: jiff::civil::Date,
) -> Option<jiff::civil::Date> {
    let p = pal(ui);
    let first = jiff::civil::Date::new(*year, *month, 1).ok()?;
    let mut picked = None;
    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
    ui.horizontal(|ui| {
        let step = |y: &mut i16, m: &mut i8, by: i8| {
            let total = *y as i32 * 12 + (*m as i32 - 1) + by as i32;
            *y = (total / 12) as i16;
            *m = (total % 12) as i8 + 1;
        };
        if ui
            .add(
                egui::Button::new("‹")
                    .frame(false)
                    .min_size(vec2(28.0, 28.0)),
            )
            .clicked()
        {
            step(year, month, -1);
        }
        ui.add_sized(
            [170.0, 28.0],
            egui::Label::new(
                egui::RichText::new(first.strftime("%B %Y").to_string())
                    .font(theme::semi(15.0))
                    .color(p.ink),
            ),
        );
        if ui
            .add(
                egui::Button::new("›")
                    .frame(false)
                    .min_size(vec2(28.0, 28.0)),
            )
            .clicked()
        {
            step(year, month, 1);
        }
    });
    let today = jiff::Zoned::now().date();
    let cell = vec2(32.0, 28.0);
    egui::Grid::new(ui.id().with("days"))
        .spacing(vec2(2.0, 2.0))
        .show(ui, |ui| {
            for d in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
                ui.add_sized(
                    cell,
                    egui::Label::new(egui::RichText::new(d).font(theme::body(12.0)).color(p.ink2)),
                );
            }
            ui.end_row();
            let lead = first.weekday().to_monday_zero_offset() as i32;
            let mut day = first.checked_sub(jiff::Span::new().days(lead)).ok();
            for _ in 0..6 {
                for _ in 0..7 {
                    let Some(d) = day else { break };
                    let in_month = d.month() == first.month();
                    let on = d == selected;
                    let (rect, resp) = ui.allocate_exact_size(cell, Sense::click());
                    let fill = if on {
                        p.trace
                    } else if resp.hovered() {
                        p.faint
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(RADIUS), fill);
                    if d == today && !on {
                        ui.painter().rect_stroke(
                            rect.shrink(1.0),
                            CornerRadius::same(RADIUS),
                            Stroke::new(1.0, p.trace),
                            StrokeKind::Inside,
                        );
                    }
                    let color = if on {
                        if p.dark {
                            p.paper
                        } else {
                            Color32::WHITE
                        }
                    } else if in_month {
                        p.ink
                    } else {
                        alpha(p.ink2, 0.5)
                    };
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        d.day().to_string(),
                        theme::medium(14.0),
                        color,
                    );
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        picked = Some(d);
                    }
                    day = d.tomorrow().ok();
                }
                ui.end_row();
            }
        });
    picked
}

/// A drop-down of `options`, showing the chosen one's label.
pub fn select<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: &str,
    value: &mut T,
    options: &[(T, String)],
    width: f32,
) -> bool {
    let current = options
        .iter()
        .find(|o| o.0 == *value)
        .map(|o| o.1.clone())
        .unwrap_or_default();
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(egui::RichText::new(current).font(theme::medium(14.0)))
        .width(width)
        .height(400.0)
        .show_ui(ui, |ui| {
            for (v, label) in options {
                if ui
                    .selectable_label(
                        *v == *value,
                        egui::RichText::new(label).font(theme::body(14.0)),
                    )
                    .clicked()
                    && *v != *value
                {
                    *value = *v;
                    changed = true;
                }
            }
        });
    changed
}

/// One figure in a [`stats`] row.
pub struct Stat {
    pub label: String,
    pub value: String,
    pub color: Color32,
    pub detail: String,
    pub detail_color: Color32,
}

impl Stat {
    pub fn new(label: impl Into<String>, value: impl Into<String>, color: Color32) -> Stat {
        Stat {
            label: label.into(),
            value: value.into(),
            color,
            detail: String::new(),
            detail_color: Color32::PLACEHOLDER,
        }
    }

    pub fn detail(mut self, d: impl Into<String>, color: Color32) -> Stat {
        self.detail = d.into();
        self.detail_color = color;
        self
    }
}

/// Figures in ruled rows, a label over each large value and a line under it.
/// Four to a row on a wide screen, two on a narrow one.
pub fn stats(ui: &mut Ui, stats: &[Stat]) {
    let p = pal(ui);
    let cols = if ui.available_width() >= 760.0 { 4 } else { 2 };
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        rule(ui, p.grid);
        for row in stats.chunks(cols) {
            columns(ui, cols, 32.0, true, |i, ui| {
                let Some(s) = row.get(i) else { return };
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add_space(10.0);
                small(ui, s.label.as_str(), p.ink2);
                text(ui, s.value.as_str(), theme::display(27.0), s.color);
                let dc = if s.detail_color == Color32::PLACEHOLDER {
                    p.ink2
                } else {
                    s.detail_color
                };
                small(ui, s.detail.as_str(), dc);
                ui.add_space(10.0);
            });
            rule(ui, p.grid);
        }
    });
}

/// A text link in the violet pen colour.
pub fn link(ui: &mut Ui, label: &str) -> Response {
    let p = pal(ui);
    let r = ui.add(
        egui::Label::new(
            egui::RichText::new(label)
                .font(theme::medium(14.0))
                .color(p.trace),
        )
        .sense(Sense::click()),
    );
    r.on_hover_cursor(CursorIcon::PointingHand)
}

/// A small coloured square for an item's kind.
pub fn kind_mark(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 16.0), Sense::hover());
    ui.painter().rect_filled(
        Rect::from_center_size(rect.center(), vec2(9.0, 9.0)),
        2,
        color,
    );
}

/// A text box for something that isn't text (an amount, a time): shows
/// `shown` until it's being typed in, then keeps what's typed until the
/// box is left. Returns the text when it changed.
pub fn typed_input(
    ui: &mut Ui,
    id: egui::Id,
    shown: String,
    hint: &str,
    width: f32,
) -> (Response, Option<String>) {
    let p = pal(ui);
    let key = id.with("typing");
    let mut buf: String = ui.data(|d| d.get_temp::<String>(key)).unwrap_or(shown);
    let r = ui.add(
        egui::TextEdit::singleline(&mut buf)
            .id(id)
            .hint_text(egui::RichText::new(hint).color(alpha(p.ink2, 0.8)))
            .font(theme::medium(14.0))
            .desired_width(width)
            .margin(Margin::symmetric(10, 8)),
    );
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(key, buf.clone()));
    } else {
        ui.data_mut(|d| d.remove::<String>(key));
    }
    let changed = r.changed().then_some(buf);
    (r, changed)
}

/// An amount, or nothing. Shows it grouped ("92,000").
pub fn amount_input(
    ui: &mut Ui,
    id: egui::Id,
    value: &mut Option<f64>,
    decimals: u8,
    width: f32,
) -> Response {
    let shown = value
        .map(|v| waypoint_core::money::group(v, decimals))
        .unwrap_or_default();
    let (r, typed) = typed_input(ui, id, shown, "0", width);
    if let Some(t) = typed {
        *value = if t.trim().is_empty() {
            None
        } else {
            waypoint_core::money::parse_amount(&t).or(*value)
        };
    }
    r
}

/// Reads "14:30", "1430", "9", "9:05 pm".
pub fn parse_time(s: &str) -> Option<jiff::civil::Time> {
    let s = s.trim().to_lowercase();
    let (s, pm, am) = match (s.strip_suffix("pm"), s.strip_suffix("am")) {
        (Some(x), _) => (x.trim().to_string(), true, false),
        (_, Some(x)) => (x.trim().to_string(), false, true),
        _ => (s, false, false),
    };
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    let (h, m) = if let Some((a, b)) = s.split_once([':', '.']) {
        (a.trim().parse::<i8>().ok()?, b.trim().parse::<i8>().ok()?)
    } else {
        match digits.len() {
            1 | 2 => (digits.parse().ok()?, 0),
            3 => (digits[..1].parse().ok()?, digits[1..].parse().ok()?),
            4 => (digits[..2].parse().ok()?, digits[2..].parse().ok()?),
            _ => return None,
        }
    };
    let h = match (h, pm, am) {
        (12, false, true) => 0,
        (h, true, _) if h < 12 => h + 12,
        (h, _, _) => h,
    };
    jiff::civil::Time::new(h, m, 0, 0).ok()
}

/// A time of day, or none.
pub fn time_input(ui: &mut Ui, id: egui::Id, value: &mut Option<jiff::civil::Time>) -> Response {
    let shown = value
        .map(|t| t.strftime("%H:%M").to_string())
        .unwrap_or_default();
    let (r, typed) = typed_input(ui, id, shown, "hh:mm", 74.0);
    if let Some(t) = typed {
        if t.trim().is_empty() {
            *value = None;
        } else if let Some(v) = parse_time(&t) {
            *value = Some(v);
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::time;

    #[test]
    fn reads_dates_people_type() {
        use jiff::civil::date;
        let like = date(2026, 11, 1);
        assert_eq!(parse_date("14/5", like), Some(date(2026, 5, 14)));
        assert_eq!(parse_date("14/5/27", like), Some(date(2027, 5, 14)));
        assert_eq!(parse_date("14 May", like), Some(date(2026, 5, 14)));
        assert_eq!(parse_date("Sat 14 May 2027", like), Some(date(2027, 5, 14)));
        assert_eq!(parse_date("12 november", like), Some(date(2026, 11, 12)));
        assert_eq!(parse_date("2027-05-14", like), Some(date(2027, 5, 14)));
        assert_eq!(parse_date("31/2", like), None);
        assert_eq!(parse_date("14", like), None);
        assert_eq!(parse_date("soon", like), None);
    }

    #[test]
    fn reads_times_people_type() {
        assert_eq!(parse_time("14:30"), Some(time(14, 30, 0, 0)));
        assert_eq!(parse_time("1430"), Some(time(14, 30, 0, 0)));
        assert_eq!(parse_time("9"), Some(time(9, 0, 0, 0)));
        assert_eq!(parse_time("9:05 pm"), Some(time(21, 5, 0, 0)));
        assert_eq!(parse_time("12am"), Some(time(0, 0, 0, 0)));
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(parse_time("soon"), None);
    }
}
