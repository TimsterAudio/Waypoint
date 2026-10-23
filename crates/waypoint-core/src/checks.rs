//! Things worth a look before you go: nights with nowhere to stay, clashes,
//! money falling due, free cancellation running out.

use jiff::civil::Date;

use crate::model::{Item, Kind, Status, Trip};

/// How soon a balance due counts as "soon".
pub const DUE_SOON_DAYS: i32 = 14;
/// How soon the end of free cancellation counts as "soon".
pub const CANCEL_SOON_DAYS: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Needs doing.
    Act,
    /// Worth a look.
    Check,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub severity: Severity,
    pub text: String,
    /// The day it's about, for jumping to the itinerary.
    pub day: Option<Date>,
    /// The items it's about, for opening them.
    pub items: Vec<String>,
}

fn w(severity: Severity, text: String, day: Option<Date>, items: Vec<String>) -> Warning {
    Warning {
        severity,
        text,
        day,
        items,
    }
}

/// A day as "Fri 14 May".
pub fn day_label(d: Date) -> String {
    d.strftime("%a %-d %b").to_string()
}

fn days_until(today: Date, d: Date) -> i32 {
    today.until(d).map(|s| s.get_days()).unwrap_or(0)
}

/// Every warning for the trip, the most pressing first.
pub fn check(trip: &Trip, today: Date) -> Vec<Warning> {
    let mut out = Vec::new();
    nights(trip, &mut out);
    clashes(trip, &mut out);
    money(trip, today, &mut out);
    details(trip, &mut out);
    out.sort_by_key(|w| (w.severity, w.day));
    out
}

/// Nights between the trip's start and end with no stay, or only ideas, or
/// two stays.
fn nights(trip: &Trip, out: &mut Vec<Warning>) {
    let (Some(start), Some(end)) = (trip.start, trip.end) else {
        return;
    };
    let mut missing: Vec<Date> = Vec::new();
    let flush = |missing: &mut Vec<Date>, out: &mut Vec<Warning>| {
        if missing.is_empty() {
            return;
        }
        let text = if missing.len() == 1 {
            format!("Nowhere to stay on the night of {}", day_label(missing[0]))
        } else {
            format!(
                "Nowhere to stay for {} nights, {} to {}",
                missing.len(),
                day_label(missing[0]),
                day_label(*missing.last().unwrap())
            )
        };
        out.push(w(Severity::Act, text, Some(missing[0]), Vec::new()));
        missing.clear();
    };
    let mut d = start;
    // The last night is the one before the trip ends.
    while d < end {
        // Overnight travel (a red-eye flight, a sleeper train) counts as a
        // bed for the night.
        let on_the_move = !trip.travelling_overnight(d).is_empty();
        let stays = trip.stays_for_night(d);
        let booked: Vec<&&Item> = stays.iter().filter(|s| s.status.committed()).collect();
        if stays.is_empty() && !on_the_move {
            missing.push(d);
        } else {
            flush(&mut missing, out);
            if booked.is_empty() && !on_the_move {
                out.push(w(
                    Severity::Check,
                    format!(
                        "Only ideas for the night of {}: {}",
                        day_label(d),
                        stays
                            .iter()
                            .map(|s| s.display_title())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    Some(d),
                    stays.iter().map(|s| s.id.clone()).collect(),
                ));
            } else if booked.len() > 1 {
                out.push(w(
                    Severity::Check,
                    format!(
                        "Two places booked for the night of {}: {}",
                        day_label(d),
                        booked
                            .iter()
                            .map(|s| s.display_title())
                            .collect::<Vec<_>>()
                            .join(" and ")
                    ),
                    Some(d),
                    booked.iter().map(|s| s.id.clone()).collect(),
                ));
            }
        }
        match d.tomorrow() {
            Ok(n) => d = n,
            Err(_) => break,
        }
    }
    flush(&mut missing, out);
}

/// Timed, booked items (not stays) whose times overlap.
fn clashes(trip: &Trip, out: &mut Vec<Warning>) {
    // Start and end as moments, each in the zone it happens in.
    let tl = trip.timeline();
    let span = |i: &Item| -> Option<(i64, i64)> {
        let s = i.start.as_ref()?;
        s.time?;
        let start = tl.instant_in(i, false)?.as_second();
        let end = i
            .end
            .as_ref()
            .filter(|e| e.time.is_some())
            .and_then(|_| tl.instant_in(i, true))
            .map(|t| t.as_second())
            .filter(|e| *e > start)
            .unwrap_or(start + 60 * 60);
        Some((start, end))
    };
    let timed: Vec<(&Item, (i64, i64))> = trip
        .items
        .iter()
        .filter(|i| i.kind != Kind::Stay && i.status.committed())
        .filter_map(|i| span(i).map(|s| (i, s)))
        .collect();
    for (n, (a, sa)) in timed.iter().enumerate() {
        for (b, sb) in timed.iter().skip(n + 1) {
            if sa.0 < sb.1 && sb.0 < sa.1 {
                out.push(w(
                    Severity::Check,
                    format!(
                        "{} and {} overlap on {}",
                        a.display_title(),
                        b.display_title(),
                        day_label(a.date().unwrap())
                    ),
                    a.date(),
                    vec![a.id.clone(), b.id.clone()],
                ));
            }
        }
    }
}

fn money(trip: &Trip, today: Date, out: &mut Vec<Warning>) {
    for i in &trip.items {
        if i.status == Status::Cancelled {
            continue;
        }
        let code = trip.currency_of(i);
        if i.status.committed() && i.cost.owing() > 0.0 {
            if let Some(due) = i.cost.due {
                let n = days_until(today, due);
                if n <= DUE_SOON_DAYS {
                    let when = match n {
                        n if n < 0 => format!("was due {}", day_label(due)),
                        0 => "due today".into(),
                        1 => "due tomorrow".into(),
                        n => format!("due in {n} days, {}", day_label(due)),
                    };
                    out.push(w(
                        Severity::Act,
                        format!(
                            "{} to pay for {}, {when}",
                            crate::money::format(i.cost.owing(), &code),
                            i.display_title()
                        ),
                        i.date(),
                        vec![i.id.clone()],
                    ));
                }
            }
        }
        if let Some(by) = i.cost.cancel_by {
            let n = days_until(today, by);
            if (0..=CANCEL_SOON_DAYS).contains(&n) {
                let when = match n {
                    0 => "today".to_string(),
                    1 => "tomorrow".to_string(),
                    n => format!("in {n} days, {}", day_label(by)),
                };
                out.push(w(
                    Severity::Act,
                    format!("Free cancellation for {} ends {when}", i.display_title()),
                    i.date(),
                    vec![i.id.clone()],
                ));
            }
        }
    }
    for code in trip.foreign_currencies() {
        let needs = trip.items.iter().any(|i| {
            trip.currency_of(i) == code && i.cost.locked_rate.is_none() && !i.cost.is_empty()
        });
        if needs && !trip.rates.get(&code).is_some_and(|r| r.home_per_unit > 0.0) {
            out.push(w(
                Severity::Act,
                format!("No exchange rate for {code} yet, so those costs aren't in the totals"),
                None,
                Vec::new(),
            ));
        }
    }
}

/// Booked items missing what you'd need on the day.
fn details(trip: &Trip, out: &mut Vec<Warning>) {
    let tl = trip.timeline();
    for i in &trip.items {
        for (w, what, is_end) in [(&i.start, "start", false), (&i.end, "end", true)] {
            let Some(w) = w else { continue };
            if i.status != Status::Cancelled
                && tl.place_as(w, i.kind, is_end).how == crate::zones::How::Ambiguous
            {
                out.push(self::w(
                    Severity::Check,
                    format!(
                        "Set a time zone for the {what} of {}: on {} that time comes both before and after crossing time zones",
                        i.display_title(),
                        day_label(w.date)
                    ),
                    Some(w.date),
                    vec![i.id.clone()],
                ));
            }
        }
        if !i.status.committed() {
            continue;
        }
        if i.reference.trim().is_empty() && matches!(i.kind, Kind::Stay | Kind::Travel) {
            out.push(w(
                Severity::Check,
                format!("{} has no booking reference", i.display_title()),
                i.date(),
                vec![i.id.clone()],
            ));
        }
        if i.start.is_none() {
            out.push(w(
                Severity::Check,
                format!(
                    "{} is {} but has no date",
                    i.display_title(),
                    i.status.label().to_lowercase()
                ),
                None,
                vec![i.id.clone()],
            ));
        }
        if let (Some(d), Some(s), Some(e)) = (i.date(), trip.start, trip.end) {
            if d < s || d > e {
                out.push(w(
                    Severity::Check,
                    format!(
                        "{} is on {}, outside the trip's dates",
                        i.display_title(),
                        day_label(d)
                    ),
                    Some(d),
                    vec![i.id.clone()],
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Item, When};
    use jiff::civil::{date, time};

    fn stay(from: Date, to: Date, status: Status) -> Item {
        let mut s = Item::new(Kind::Stay);
        s.title = format!("Hotel {from}");
        s.status = status;
        s.reference = "R".into();
        s.start = Some(When::day(from));
        s.end = Some(When::day(to));
        s
    }

    fn trip() -> Trip {
        let mut t = Trip::new("T");
        t.start = Some(date(2027, 5, 1));
        t.end = Some(date(2027, 5, 6));
        t
    }

    #[test]
    fn finds_a_gap_in_the_nights() {
        let mut t = trip();
        t.items
            .push(stay(date(2027, 5, 1), date(2027, 5, 3), Status::Booked));
        t.items
            .push(stay(date(2027, 5, 4), date(2027, 5, 6), Status::Booked));
        let w = check(&t, date(2027, 1, 1));
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].text.contains("night of Mon 3 May"), "{}", w[0].text);
    }

    #[test]
    fn overnight_travel_counts_as_a_bed() {
        let mut t = trip();
        t.items
            .push(stay(date(2027, 5, 1), date(2027, 5, 3), Status::Booked));
        t.items
            .push(stay(date(2027, 5, 4), date(2027, 5, 6), Status::Booked));
        let mut f = Item::new(Kind::Travel);
        f.status = Status::Booked;
        f.reference = "F".into();
        f.start = Some(When::at(date(2027, 5, 3), time(22, 0, 0, 0)));
        f.end = Some(When::at(date(2027, 5, 4), time(6, 0, 0, 0)));
        t.items.push(f);
        assert!(check(&t, date(2027, 1, 1)).is_empty());
    }

    #[test]
    fn a_flight_over_the_date_line_covers_both_nights() {
        let mut t = trip();
        t.items
            .push(stay(date(2027, 5, 1), date(2027, 5, 3), Status::Booked));
        t.items
            .push(stay(date(2027, 5, 5), date(2027, 5, 6), Status::Booked));
        // Leaves the evening of the 3rd, lands the morning of the 5th.
        let mut f = Item::new(Kind::Travel);
        f.status = Status::Booked;
        f.reference = "F".into();
        f.start = Some(When::at(date(2027, 5, 3), time(22, 0, 0, 0)).in_tz("America/Los_Angeles"));
        f.end = Some(When::at(date(2027, 5, 5), time(6, 0, 0, 0)).in_tz("Pacific/Auckland"));
        t.items.push(f);
        assert!(
            check(&t, date(2027, 1, 1)).is_empty(),
            "{:?}",
            check(&t, date(2027, 1, 1))
        );
    }

    #[test]
    fn ideas_only_and_double_bookings() {
        let mut t = trip();
        t.end = Some(date(2027, 5, 3));
        t.items
            .push(stay(date(2027, 5, 1), date(2027, 5, 2), Status::Idea));
        t.items
            .push(stay(date(2027, 5, 2), date(2027, 5, 3), Status::Booked));
        t.items
            .push(stay(date(2027, 5, 2), date(2027, 5, 3), Status::Paid));
        let w = check(&t, date(2027, 1, 1));
        assert!(w.iter().any(|w| w.text.starts_with("Only ideas")));
        assert!(w.iter().any(|w| w.text.starts_with("Two places")));
    }

    #[test]
    fn money_due_and_cancellation() {
        let mut t = trip();
        t.start = None;
        let mut i = stay(date(2027, 5, 1), date(2027, 5, 2), Status::Confirmed);
        i.cost.amount = 500.0;
        i.cost.paid = 100.0;
        i.cost.due = Some(date(2027, 4, 10));
        i.cost.cancel_by = Some(date(2027, 4, 2));
        t.items.push(i);
        let w = check(&t, date(2027, 4, 1));
        assert!(
            w.iter().any(|w| w.text
                == "$400.00 to pay for Hotel 2027-05-01, due in 9 days, Sat 10 Apr"),
            "{w:?}"
        );
        assert!(w.iter().any(|w| w.text.ends_with("ends tomorrow")), "{w:?}");
    }

    #[test]
    fn clashing_times() {
        let mut t = trip();
        t.start = None;
        for (h, title) in [(10, "Tea ceremony"), (11, "Sumo")] {
            let mut i = Item::new(Kind::Activity);
            i.title = title.into();
            i.status = Status::Booked;
            i.start = Some(When::at(date(2027, 5, 2), time(h, 0, 0, 0)));
            i.end = Some(When::at(date(2027, 5, 2), time(h + 2, 0, 0, 0)));
            t.items.push(i);
        }
        let w = check(&t, date(2027, 1, 1));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].text, "Tea ceremony and Sumo overlap on Sun 2 May");
    }
}
