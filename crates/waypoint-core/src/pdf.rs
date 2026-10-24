//! The itinerary as a PDF, made by Waypoint itself: day by day, each booking
//! with its details, and where you sleep each night. Either everything (for
//! you) or the version to share (built from the same private-free copy as
//! the shared page). A4, in the Barlow fonts the app uses.

use genpdf::elements::{Break, LinearLayout, Paragraph, TableLayout};
use genpdf::style::{Color, Style};
use genpdf::{fonts, Alignment};

use crate::details::{self, time_of, Tonight};
use crate::model::{Status, Trip};

const REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Barlow-Regular.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Barlow-SemiBold.ttf");

/// Waypoint's slate ink, grey and violet, for paper.
const INK: Color = Color::Rgb(0x1c, 0x26, 0x33);
const GREY: Color = Color::Rgb(0x5a, 0x65, 0x70);
const VIOLET: Color = Color::Rgb(0x5b, 0x3a, 0x9e);
const AMBER: Color = Color::Rgb(0xa8, 0x70, 0x0c);

fn font(bytes: &[u8]) -> fonts::FontData {
    fonts::FontData::new(bytes.to_vec(), None).expect("the built-in fonts load")
}

fn ink() -> Style {
    Style::new().with_color(INK)
}

fn grey() -> Style {
    Style::new().with_color(GREY)
}

/// The PDF's bytes. `shared` leaves out everything private, as the page to
/// share does. `now` is printed as when it was made.
pub fn render(trip: &Trip, now: &jiff::Zoned, shared: bool) -> Result<Vec<u8>, String> {
    let copy;
    let trip = if shared {
        copy = details::shareable(trip);
        &copy
    } else {
        trip
    };
    let family = fonts::FontFamily {
        regular: font(REGULAR),
        bold: font(SEMIBOLD),
        italic: font(REGULAR),
        bold_italic: font(SEMIBOLD),
    };
    let mut doc = genpdf::Document::new(family);
    doc.set_title(trip.name.clone());
    doc.set_paper_size(genpdf::PaperSize::A4);
    doc.set_font_size(10);
    doc.set_line_spacing(1.2);
    let mut deco = genpdf::SimplePageDecorator::new();
    deco.set_margins(genpdf::Margins::trbl(12, 16, 14, 16));
    let name = trip.name.clone();
    deco.set_header(move |page| {
        let mut l = LinearLayout::vertical();
        if page > 1 {
            l.push(
                Paragraph::default()
                    .styled_string(format!("{name} · page {page}"), grey().with_font_size(8))
                    .aligned(Alignment::Right),
            );
            l.push(Break::new(1));
        }
        l
    });
    doc.set_page_decorator(deco);

    doc.push(Paragraph::default().styled_string(&trip.name, ink().bold().with_font_size(22)));
    let mut sub = Vec::new();
    if let (Some(s), Some(e)) = (trip.start, trip.end) {
        let nights = s.until(e).map(|sp| sp.get_days()).unwrap_or(0);
        sub.push(format!(
            "{} to {} · {nights} nights",
            s.strftime("%a %-d %b %Y"),
            e.strftime("%a %-d %b %Y")
        ));
    }
    if !trip.travellers.is_empty() {
        sub.push(trip.travellers.join(", "));
    }
    if !sub.is_empty() {
        doc.push(Paragraph::default().styled_string(sub.join(" · "), grey().with_font_size(11)));
    }
    if !shared {
        let t = trip.totals();
        let home = &trip.home_currency;
        doc.push(Break::new(0.6));
        doc.push(
            Paragraph::default()
                .styled_string("Booked ", grey())
                .styled_string(crate::money::format(t.committed, home), ink().bold())
                .styled_string("   Paid ", grey())
                .styled_string(crate::money::format(t.paid, home), ink().bold())
                .styled_string("   Still to pay ", grey())
                .styled_string(
                    crate::money::format(t.owing(), home),
                    if t.owing() > 0.0 {
                        Style::new().bold().with_color(AMBER)
                    } else {
                        ink().bold()
                    },
                ),
        );
    }

    let tl = trip.timeline();
    for d in trip.days() {
        doc.push(Break::new(1.2));
        doc.push(Paragraph::default().styled_string(
            d.strftime("%A %-d %B").to_string(),
            Style::new().bold().with_font_size(14).with_color(VIOLET),
        ));
        doc.push(Break::new(0.3));
        for i in trip.items_on(d) {
            doc.push(item(trip, &tl, i, shared));
            doc.push(Break::new(0.4));
        }
        let tonight = match details::tonight(trip, d) {
            Tonight::Stays(names) => Some((format!("Tonight: {}", names.join(" or ")), grey())),
            Tonight::Travelling => Some(("Tonight: travelling overnight".to_string(), grey())),
            Tonight::Missing if !shared => Some((
                "Tonight: nowhere booked yet".to_string(),
                Style::new().with_color(AMBER),
            )),
            _ => None,
        };
        if let Some((t, style)) = tonight {
            doc.push(Paragraph::default().styled_string(t, style.italic()));
        }
    }

    let undated = trip.undated();
    if !undated.is_empty() {
        doc.push(Break::new(1.2));
        doc.push(Paragraph::default().styled_string(
            "No date yet",
            Style::new().bold().with_font_size(14).with_color(VIOLET),
        ));
        doc.push(Break::new(0.3));
        for i in undated {
            doc.push(item(trip, &tl, i, shared));
            doc.push(Break::new(0.4));
        }
    }
    if !shared && !trip.notes.trim().is_empty() {
        doc.push(Break::new(1.2));
        doc.push(Paragraph::default().styled_string(
            "Notes",
            Style::new().bold().with_font_size(14).with_color(VIOLET),
        ));
        for line in trip.notes.lines() {
            doc.push(Paragraph::default().styled_string(line, ink()));
        }
    }

    doc.push(Break::new(1.5));
    let footer = if shared {
        format!(
            "Our plans as of {}. Times are local to each place; home times are {}.",
            now.strftime("%-d %B %Y"),
            tl.home_name()
        )
    } else {
        format!(
            "Made by Waypoint on {} at {}.",
            now.strftime("%a %-d %b %Y"),
            now.strftime("%H:%M")
        )
    };
    doc.push(Paragraph::default().styled_string(footer, grey().with_font_size(8)));

    let mut out = Vec::new();
    doc.render(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// One booking: the time on the left; the title, then its details.
fn item(
    trip: &Trip,
    tl: &crate::zones::Timeline,
    i: &crate::model::Item,
    shared: bool,
) -> TableLayout {
    let mut table = TableLayout::new(vec![1, 8]);
    let time = i.start.as_ref().map(time_of).unwrap_or_default();
    let mut right = LinearLayout::vertical();
    let cancelled = i.status == Status::Cancelled;
    let mut title = Paragraph::default().styled_string(
        i.display_title(),
        if cancelled {
            grey().bold()
        } else {
            ink().bold()
        }
        .with_font_size(11),
    );
    if !shared {
        title = title.styled_string(format!("   {}", i.status.label()), grey().with_font_size(9));
    }
    right.push(title);
    for r in details::rows(trip, tl, i) {
        let mut p = Paragraph::default();
        if !r.label.is_empty() {
            p = p.styled_string(format!("{}  ", r.label), grey());
        }
        let style = match r.value {
            details::Value::Reference(_) => ink().bold(),
            details::Value::Cost { owing: Some(_), .. } => Style::new().with_color(AMBER),
            _ => ink(),
        };
        right.push(p.styled_string(r.value.plain(), style));
    }
    if !shared && !i.notes.trim().is_empty() {
        for line in i.notes.trim().lines() {
            right.push(Paragraph::default().styled_string(line, grey().italic()));
        }
    }
    table
        .row()
        .element(Paragraph::default().styled_string(
            time,
            Style::new().bold().with_font_size(11).with_color(VIOLET),
        ))
        .element(right)
        .push()
        .expect("two cells for two columns");
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> jiff::Zoned {
        jiff::civil::date(2027, 3, 1)
            .at(9, 0, 0, 0)
            .in_tz("Australia/Sydney")
            .unwrap()
    }

    #[test]
    fn makes_a_pdf_for_both() {
        let t = crate::sample::trip();
        for shared in [false, true] {
            let pdf = render(&t, &now(), shared).unwrap();
            assert!(pdf.starts_with(b"%PDF-"), "not a PDF");
            assert!(pdf.len() > 10_000);
        }
    }
}
