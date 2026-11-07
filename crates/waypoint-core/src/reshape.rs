//! Reshaping a trip while it's being planned: reversing it end to end, or
//! swapping two of its stays. Both keep the trip's first and last days and
//! its length, and the app offers them only while `Trip::can_move`. Payment
//! and cancellation dates stay put, as they do when the trip moves.

use jiff::civil::Date;

use crate::model::{Item, Kind, Status, Travel, Trip, When};
use crate::places::town_in;

fn add(d: Date, days: i64) -> Date {
    d.checked_add(jiff::Span::new().days(days)).unwrap_or(d)
}

fn days(a: Date, b: Date) -> i64 {
    a.until(b).map(|s| s.get_days() as i64).unwrap_or(0)
}

/// A place's name without its airport code: "Da Nang (DAD)" is "Da Nang".
fn bare(text: &str) -> &str {
    text.split('(').next().unwrap_or(text).trim()
}

/// The town a journey's end names, or the end as typed.
fn town_of(text: &str) -> String {
    town_in(text)
        .map(str::to_string)
        .unwrap_or_else(|| bare(text).to_string())
        .to_lowercase()
}

/// Whether a journey's notes name a way through ("Connecting in Bangkok",
/// "via Singapore"), which may not be right once it goes somewhere else.
fn names_a_connection(notes: &str) -> bool {
    let n = notes.to_lowercase();
    [
        "connect",
        "via ",
        "transit",
        "change at",
        "change in",
        "stopover",
        "layover",
    ]
    .iter()
    .any(|w| n.contains(w))
}

/// Give a journey new ends, and its title too where it names the old ones
/// ("Fly Melbourne to Da Nang").
fn repoint(title: &mut String, t: &mut Travel, from: String, to: String) {
    let (old_from, old_to) = (bare(&t.from).to_string(), bare(&t.to).to_string());
    let (new_from, new_to) = (bare(&from).to_string(), bare(&to).to_string());
    if !old_from.is_empty() && !old_to.is_empty() {
        let old = format!("{old_from} to {old_to}");
        if title.contains(&old) {
            *title = title.replace(&old, &format!("{new_from} to {new_to}"));
        }
    }
    t.from = from;
    t.to = to;
}

/// Reverse the trip end to end: what happened on the last day happens on
/// the first, journeys go the other way, and each stay keeps its nights.
/// Times are cleared, since a 06:00 departure means nothing flown the other
/// way. Needs the trip's dates; returns false without them.
pub fn reverse(trip: &mut Trip) -> bool {
    let (Some(start), Some(end)) = (trip.start, trip.end) else {
        return false;
    };
    let flip = |w: When| When {
        date: add(start, days(w.date, end)),
        time: None,
        tz: w.tz,
    };
    for i in &mut trip.items {
        match (i.start.take(), i.end.take()) {
            (Some(s), Some(e)) => {
                i.start = Some(flip(e));
                i.end = Some(flip(s));
            }
            (Some(s), None) => i.start = Some(flip(s)),
            (None, e) => i.end = e,
        }
        if let Some(t) = i.travel.as_mut() {
            let (from, to) = (t.from.clone(), t.to.clone());
            repoint(&mut i.title, t, to, from);
        }
    }
    true
}

/// The stays a swap can choose between: dated, not cancelled, in order.
pub fn stays(trip: &Trip) -> Vec<&Item> {
    let mut v: Vec<&Item> = trip
        .items
        .iter()
        .filter(|i| {
            i.kind == Kind::Stay
                && i.status != Status::Cancelled
                && i.date().is_some()
                && i.end.as_ref().is_some_and(|e| Some(e.date) > i.date())
        })
        .collect();
    v.sort_by_key(|i| i.date());
    v
}

/// One stay in the stretch being swapped.
struct Stop {
    id: String,
    town: String,
    check_in: Date,
    check_out: Date,
    /// What the journeys arriving and leaving call it ("Da Nang (DAD)").
    arrive: String,
    leave: String,
}

/// Swap two stays: each takes the other's place in the route with its own
/// nights and the things planned during it, the stays between them move
/// along to make room, and the journeys between are re-pointed. Returns
/// the journeys worth a look afterwards: ones whose notes name a
/// connection that may no longer be right, or that couldn't be re-pointed.
pub fn swap(trip: &mut Trip, a: &str, b: &str) -> Result<Vec<String>, String> {
    let order = stays(trip);
    let pos = |id: &str| order.iter().position(|i| i.id == id);
    let (Some(pa), Some(pb)) = (pos(a), pos(b)) else {
        return Err("Choose two stays with their dates.".into());
    };
    if pa == pb {
        return Err("Choose two different stays.".into());
    }
    let (lo, hi) = (pa.min(pb), pa.max(pb));
    let journeys_on = |d: Date| -> Vec<&Item> {
        trip.items
            .iter()
            .filter(|i| i.kind == Kind::Travel && i.date() == Some(d))
            .collect()
    };
    let mut stops: Vec<Stop> = Vec::new();
    for s in &order[lo..=hi] {
        let (check_in, check_out) = (s.date().unwrap(), s.end.as_ref().unwrap().date);
        let town = s.town().to_lowercase();
        let name = |pick: fn(&Travel) -> &str, d: Date| {
            journeys_on(d)
                .iter()
                .filter_map(|j| j.travel.as_ref().map(pick))
                .find(|t| !town.is_empty() && town_of(t) == town)
                .map(str::to_string)
                .unwrap_or_else(|| s.town())
        };
        stops.push(Stop {
            id: s.id.clone(),
            arrive: name(|t| &t.to, check_in),
            leave: name(|t| &t.from, check_out),
            town,
            check_in,
            check_out,
        });
    }
    if stops.windows(2).any(|w| w[0].check_out > w[1].check_in) {
        return Err("Some of the stays between them overlap, so they can't trade places.".into());
    }

    // The new order, and where each stay and each changeover lands.
    let mut new_order: Vec<usize> = (0..stops.len()).collect();
    let last = new_order.len() - 1;
    new_order.swap(0, last);
    let mut new_in = vec![stops[0].check_in; stops.len()];
    let mut shift_changeover = vec![0i64; stops.len() - 1];
    let mut cursor = stops[0].check_in;
    for (k, &s) in new_order.iter().enumerate() {
        new_in[s] = cursor;
        cursor = add(cursor, days(stops[s].check_in, stops[s].check_out));
        if k < last {
            shift_changeover[k] = days(stops[k].check_out, cursor);
            cursor = add(cursor, days(stops[k].check_out, stops[k + 1].check_in));
        }
    }

    // What each item goes with: a stay, a changeover between two stays,
    // the journey into the first or out of the last, or nothing here.
    enum Part {
        Stay(usize),
        Changeover(usize),
        Into,
        OutOf,
    }
    let part_of = |i: &Item| -> Option<Part> {
        let d = i.date()?;
        if let Some(s) = stops.iter().position(|s| s.id == i.id) {
            return Some(Part::Stay(s));
        }
        if i.kind == Kind::Travel {
            if d == stops[0].check_in {
                return Some(Part::Into);
            }
            if d == stops[last].check_out {
                return Some(Part::OutOf);
            }
            if let Some(k) =
                (0..last).find(|&k| stops[k].check_out <= d && d <= stops[k + 1].check_in)
            {
                return Some(Part::Changeover(k));
            }
        }
        stops
            .iter()
            .position(|s| s.check_in <= d && d < s.check_out)
            .map(Part::Stay)
    };

    let mut check = Vec::new();
    for i in &mut trip.items {
        let Some(part) = part_of(i) else { continue };
        let shift = match part {
            Part::Stay(s) => days(stops[s].check_in, new_in[s]),
            Part::Changeover(k) => shift_changeover[k],
            Part::Into | Part::OutOf => 0,
        };
        for w in [i.start.as_mut(), i.end.as_mut()].into_iter().flatten() {
            w.date = add(w.date, shift);
        }
        // Which stays the journey now goes between: (old, new) for each end.
        let ends = match part {
            Part::Stay(_) => continue,
            Part::Changeover(k) => (Some((k, new_order[k])), Some((k + 1, new_order[k + 1]))),
            Part::Into => (None, Some((0, new_order[0]))),
            Part::OutOf => (Some((last, new_order[last])), None),
        };
        let Some(t) = i.travel.as_mut() else { continue };
        let (mut from, mut to) = (t.from.clone(), t.to.clone());
        let mut moved = 0;
        if let Some((old, new)) = ends.0 {
            if !stops[old].town.is_empty() && town_of(&from) == stops[old].town {
                from = stops[new].leave.clone();
                moved += 1;
            }
        }
        if let Some((old, new)) = ends.1 {
            if !stops[old].town.is_empty() && town_of(&to) == stops[old].town {
                to = stops[new].arrive.clone();
                moved += 1;
            }
        }
        repoint(&mut i.title, t, from, to);
        for w in [i.start.as_mut(), i.end.as_mut()].into_iter().flatten() {
            w.time = None;
        }
        if moved == 0 || names_a_connection(&i.notes) {
            check.push(i.display_title());
        }
    }
    Ok(check)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mode;
    use jiff::civil::date;

    fn item(kind: Kind, title: &str, from: Date, to: Date) -> Item {
        let mut i = Item::new(kind);
        i.title = title.into();
        i.start = Some(When::day(from));
        i.end = Some(When::day(to));
        i
    }

    fn flight(title: &str, d: Date, e: Date, from: &str, to: &str) -> Item {
        let mut i = item(Kind::Travel, title, d, e);
        i.travel = Some(Travel {
            mode: Mode::Flight,
            from: from.into(),
            to: to.into(),
            ..Default::default()
        });
        i
    }

    fn stay(town: &str, d: Date, e: Date) -> Item {
        let mut i = item(Kind::Stay, &format!("Stay in {town}"), d, e);
        i.place = format!("{town}, somewhere");
        i
    }

    /// SE Asia 2027, cut down: 5 to 26 October.
    fn trip() -> Trip {
        let d = |n| date(2027, 10, n);
        let mut t = Trip::new("SE Asia");
        t.start = Some(d(5));
        t.end = Some(d(26));
        t.items = vec![
            flight(
                "Fly Melbourne to Da Nang",
                d(5),
                d(5),
                "Melbourne (MEL)",
                "Da Nang (DAD)",
            ),
            stay("Da Nang", d(5), d(9)),
            flight(
                "Fly Da Nang to Siem Reap",
                d(9),
                d(9),
                "Da Nang (DAD)",
                "Siem Reap (SAI)",
            ),
            stay("Siem Reap", d(9), d(13)),
            flight(
                "Fly Siem Reap to Chiang Mai",
                d(13),
                d(13),
                "Siem Reap (SAI)",
                "Chiang Mai (CNX)",
            ),
            stay("Chiang Mai", d(13), d(19)),
            item(Kind::Activity, "Saturday Walking Street", d(16), d(16)),
            flight(
                "Fly Chiang Mai to Bangkok",
                d(19),
                d(19),
                "Chiang Mai (CNX)",
                "Bangkok (BKK)",
            ),
            stay("Bangkok", d(19), d(25)),
            flight(
                "Fly Bangkok to Melbourne",
                d(25),
                d(26),
                "Bangkok (BKK)",
                "Melbourne (MEL)",
            ),
        ];
        t
    }

    /// A fresh `trip()` with `t`'s item ids, to compare with.
    fn fresh_like(t: &Trip) -> Trip {
        let mut back = trip();
        for (b, a) in back.items.iter_mut().zip(&t.items) {
            b.id = a.id.clone();
        }
        back
    }

    fn line(i: &Item) -> String {
        let e = i.end.as_ref().map(|w| w.date.day()).unwrap_or(0);
        format!("{} {}-{}", i.display_title(), i.date().unwrap().day(), e)
    }

    #[test]
    fn reverses_end_to_end() {
        let mut t = trip();
        t.items[0].start.as_mut().unwrap().time = Some(jiff::civil::time(6, 0, 0, 0));
        assert!(reverse(&mut t));
        let mut v: Vec<&Item> = t.items.iter().collect();
        v.sort_by_key(|i| (i.date(), i.kind != Kind::Travel));
        let lines: Vec<String> = v.iter().map(|i| line(i)).collect();
        assert_eq!(
            lines,
            [
                "Fly Melbourne to Bangkok 5-6",
                "Stay in Bangkok 6-12",
                "Fly Bangkok to Chiang Mai 12-12",
                "Stay in Chiang Mai 12-18",
                "Saturday Walking Street 15-15",
                "Fly Chiang Mai to Siem Reap 18-18",
                "Stay in Siem Reap 18-22",
                "Fly Siem Reap to Da Nang 22-22",
                "Stay in Da Nang 22-26",
                "Fly Da Nang to Melbourne 26-26",
            ]
        );
        assert!(t
            .items
            .iter()
            .all(|i| i.start.as_ref().unwrap().time.is_none()));
        let first = t
            .items
            .iter()
            .find(|i| i.title.starts_with("Fly Melbourne"))
            .unwrap();
        assert_eq!(first.travel.as_ref().unwrap().to, "Bangkok (BKK)");
        // Twice is back where it started, times aside.
        assert!(reverse(&mut t));
        let mut back = fresh_like(&t);
        back.items[0].start.as_mut().unwrap().time = None;
        assert_eq!(t, back);
    }

    #[test]
    fn swaps_two_stays_and_moves_the_one_between() {
        let mut t = trip();
        let ids: Vec<String> = stays(&t).iter().map(|s| s.id.clone()).collect();
        // Siem Reap (4 nights) and Bangkok (6), with Chiang Mai between.
        let check = swap(&mut t, &ids[1], &ids[3]).unwrap();
        assert!(check.is_empty());
        let lines: Vec<String> = t.items.iter().map(line).collect();
        assert_eq!(
            lines,
            [
                "Fly Melbourne to Da Nang 5-5",
                "Stay in Da Nang 5-9",
                "Fly Da Nang to Bangkok 9-9",
                "Stay in Siem Reap 21-25",
                "Fly Bangkok to Chiang Mai 15-15",
                "Stay in Chiang Mai 15-21",
                "Saturday Walking Street 18-18",
                "Fly Chiang Mai to Siem Reap 21-21",
                "Stay in Bangkok 9-15",
                "Fly Siem Reap to Melbourne 25-26",
            ]
        );
        assert_eq!(t.items[2].travel.as_ref().unwrap().to, "Bangkok (BKK)");
        assert_eq!(t.items[9].travel.as_ref().unwrap().from, "Siem Reap (SAI)");
        // Swapping back puts it all where it was.
        swap(&mut t, &ids[1], &ids[3]).unwrap();
        assert_eq!(t, fresh_like(&t));
    }

    #[test]
    fn journeys_with_notes_are_listed_to_check() {
        let mut t = trip();
        t.items[4].notes = "Connecting in Bangkok".into();
        t.items[2].notes = "Direct. One checked bag each.".into();
        let ids: Vec<String> = stays(&t).iter().map(|s| s.id.clone()).collect();
        let check = swap(&mut t, &ids[1], &ids[2]).unwrap();
        assert_eq!(check, ["Fly Chiang Mai to Siem Reap"]);
        assert_eq!(
            swap(&mut t, &ids[1], &ids[1]),
            Err("Choose two different stays.".into())
        );
    }
}
