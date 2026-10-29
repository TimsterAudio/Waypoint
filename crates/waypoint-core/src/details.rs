//! What to show about each booking, and about each night, on a page: the
//! phone itinerary, the itinerary to share and the PDFs all list the same
//! things, each in its own way.

use jiff::civil::Date;

use crate::checks::day_label;
use crate::model::{Item, Kind, Status, Trip, When};
use crate::zones::Timeline;

/// One line of detail.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    /// A booking reference, shown so it's easy to read out.
    Reference(String),
    Address(String),
    Phone(String),
    Email(String),
    Website(String),
    /// The cost, and what's still to pay on it if anything.
    Cost {
        amount: String,
        owing: Option<String>,
        paid: bool,
    },
}

impl Value {
    /// The line as plain text.
    pub fn plain(&self) -> String {
        match self {
            Value::Text(s)
            | Value::Reference(s)
            | Value::Address(s)
            | Value::Phone(s)
            | Value::Email(s)
            | Value::Website(s) => s.clone(),
            Value::Cost {
                amount,
                owing,
                paid,
            } => match (owing, paid) {
                (Some(o), _) => format!("{amount} · {o}"),
                (None, true) => format!("{amount} · paid"),
                (None, false) => amount.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// "Check in", "Arrives", "Ref"; empty for a line with no label.
    pub label: &'static str,
    pub value: Value,
}

fn row(label: &'static str, value: Value) -> Row {
    Row { label, value }
}

fn text(label: &'static str, s: impl Into<String>) -> Row {
    row(label, Value::Text(s.into()))
}

/// "14:30", or nothing for an untimed day.
pub fn time_of(w: &When) -> String {
    w.time
        .map(|t| t.strftime("%H:%M").to_string())
        .unwrap_or_default()
}

/// The lines of detail for one booking, in the order they're shown.
pub fn rows(trip: &Trip, tl: &Timeline, i: &Item) -> Vec<Row> {
    let mut rows = Vec::new();
    match i.kind {
        Kind::Stay => {
            if let Some(s) = &i.start {
                let t = time_of(s);
                rows.push(text(
                    "Check in",
                    if t.is_empty() {
                        day_label(s.date)
                    } else {
                        format!("{} from {t}", day_label(s.date))
                    },
                ));
            }
            if let Some(e) = &i.end {
                let t = time_of(e);
                rows.push(text(
                    "Check out",
                    if t.is_empty() {
                        day_label(e.date)
                    } else {
                        format!("{} by {t}", day_label(e.date))
                    },
                ));
            }
            if !i.room.is_empty() {
                rows.push(text("Room", &i.room));
            }
        }
        Kind::Travel => {
            if let Some(t) = &i.travel {
                let what = format!("{} {} {}", t.mode.label(), t.carrier, t.number);
                rows.push(text("", what.trim()));
                if !t.from.is_empty() {
                    rows.push(text("From", &t.from));
                }
                if !t.terminal.is_empty() {
                    rows.push(text("Terminal", &t.terminal));
                }
                if !t.to.is_empty() {
                    rows.push(text("To", &t.to));
                }
                if !t.seat.is_empty() {
                    rows.push(text("Seat", &t.seat));
                }
            }
            if let Some(e) = &i.end {
                let t = time_of(e);
                let same_day = i.date() == Some(e.date);
                let mut at = match (t.is_empty(), same_day) {
                    (true, _) => day_label(e.date),
                    (false, true) => t,
                    (false, false) => format!("{t}, {}", day_label(e.date)),
                };
                if let Some(m) = tl.duration_minutes(i) {
                    at.push_str(&format!(" ({}h {:02}m)", m / 60, m % 60));
                }
                rows.push(text("Arrives", at));
            }
        }
        _ => {
            if let Some(e) = &i.end {
                let t = time_of(e);
                if !t.is_empty() {
                    rows.push(text("Until", t));
                }
            }
        }
    }
    if let Some(home) = tl.home_times(i) {
        rows.push(text("Home time", home));
    }
    if !i.reference.is_empty() {
        rows.push(row("Ref", Value::Reference(i.reference.clone())));
    }
    if !i.provider.is_empty() {
        rows.push(text("With", &i.provider));
    }
    if !i.place.is_empty() && i.place != i.display_title() {
        rows.push(text("Place", &i.place));
    }
    if !i.address.is_empty() {
        rows.push(row("Address", Value::Address(i.address.clone())));
    }
    if !i.contact.is_empty() {
        rows.push(text("Contact", &i.contact));
    }
    if !i.phone.is_empty() {
        rows.push(row("Phone", Value::Phone(i.phone.clone())));
    }
    if !i.email.is_empty() {
        rows.push(row("Email", Value::Email(i.email.clone())));
    }
    if !i.website.is_empty() {
        rows.push(row("Web", Value::Website(i.website.clone())));
    }
    if !i.cost.is_empty() {
        let code = trip.currency_of(i);
        let owing = (i.cost.owing() > 0.0 && i.status != Status::Cancelled).then(|| {
            format!(
                "{} to pay{}",
                crate::money::format(i.cost.owing(), &code),
                i.cost
                    .due
                    .map(|d| format!(" by {}", day_label(d)))
                    .unwrap_or_default()
            )
        });
        rows.push(row(
            "Cost",
            Value::Cost {
                amount: trip.cost_label(i),
                paid: owing.is_none() && i.cost.paid > 0.0,
                owing,
            },
        ));
    }
    rows
}

/// Where you sleep on the night of `day`: the stays' names, travelling
/// overnight, nowhere yet (`Missing`), or nothing to say (the last day, or
/// before the trip starts).
#[derive(Debug, Clone, PartialEq)]
pub enum Tonight {
    Stays(Vec<String>),
    Travelling,
    Missing,
    Nothing,
}

pub fn tonight(trip: &Trip, day: Date) -> Tonight {
    let last_night = trip.end.is_some_and(|e| day >= e);
    let stays = trip.stays_for_night(day);
    if !stays.is_empty() {
        Tonight::Stays(stays.iter().map(|s| s.display_title()).collect())
    } else if !last_night && !trip.travelling_overnight(day).is_empty() {
        Tonight::Travelling
    } else if !last_night && trip.start.is_some_and(|s| day >= s) {
        Tonight::Missing
    } else {
        Tonight::Nothing
    }
}

/// A copy of the trip for family and friends: booked items only, without
/// costs, booking references, who it's booked with, contacts, rooms, seats
/// or notes. Built as a copy so nothing private can reach a shared page by
/// mistake.
pub fn shareable(trip: &Trip) -> Trip {
    let mut t = Trip {
        name: trip.name.clone(),
        home_currency: trip.home_currency.clone(),
        home_tz: trip.home_tz.clone(),
        start: trip.start,
        end: trip.end,
        travellers: trip.travellers.clone(),
        ..Default::default()
    };
    t.items = trip
        .items
        .iter()
        .filter(|i| i.status.committed())
        .map(|i| {
            let mut i = i.clone();
            i.cost = Default::default();
            i.reference.clear();
            i.provider.clear();
            i.contact.clear();
            i.email.clear();
            i.room.clear();
            i.notes.clear();
            i.updated_by.clear();
            i.extra.clear();
            if let Some(tr) = &mut i.travel {
                tr.seat.clear();
            }
            i
        })
        .collect();
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_for_a_flight_and_a_hotel() {
        let t = crate::sample::trip();
        let tl = t.timeline();
        let flight = &t.items[0];
        let r = rows(&t, &tl, flight);
        assert_eq!(r[0], text("", "Flight QF 25"));
        assert!(r
            .iter()
            .any(|r| r.label == "Arrives" && r.value.plain() == "17:35 (8h 50m)"));
        assert!(r.contains(&row("Ref", Value::Reference("QX7K2P".into()))));
        let hotel = &t.items[1];
        let r = rows(&t, &tl, hotel);
        let cost = r.iter().find(|r| r.label == "Cost").unwrap();
        assert_eq!(
            cost.value.plain(),
            "¥92,000 (≈ $938.40) · ¥92,000 to pay by Sat 1 May"
        );
    }

    #[test]
    fn nights() {
        let t = crate::sample::trip();
        use jiff::civil::date;
        assert_eq!(
            tonight(&t, date(2027, 5, 16)),
            Tonight::Stays(vec!["Hotel Kumo Shinjuku".into()])
        );
        assert_eq!(tonight(&t, date(2027, 5, 23)), Tonight::Missing);
        assert_eq!(tonight(&t, date(2027, 5, 24)), Tonight::Travelling);
        assert_eq!(tonight(&t, date(2027, 5, 25)), Tonight::Nothing);
    }
}
