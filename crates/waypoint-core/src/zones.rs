//! Where you are, and so what time it is, at each point of a trip.
//!
//! Items can have their own time zone, but most won't: a hotel check-in at
//! 15:00 is simply "15:00 wherever the hotel is". Waypoint works out where
//! that is from the journeys that have zones: before the first one you're
//! at home; after each one you're in the zone it arrives in.
//!
//! Across the date line a clock time can happen twice on one day: flying
//! Auckland to Los Angeles on Tuesday evening lands on Tuesday morning, so
//! "Tuesday 12:00" is both before the flight (in Auckland) and after it (in
//! Los Angeles). An item like that without its own zone is ambiguous, and
//! the person is asked to set one.

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::Timestamp;

use crate::model::{Item, Kind, Status, Trip, When};

/// Common time zones, for zone menus. Any IANA name can be typed.
pub const COMMON: &[&str] = &[
    "Australia/Sydney",
    "Australia/Melbourne",
    "Australia/Brisbane",
    "Australia/Adelaide",
    "Australia/Perth",
    "Australia/Hobart",
    "Australia/Darwin",
    "Pacific/Auckland",
    "Pacific/Fiji",
    "Asia/Tokyo",
    "Asia/Seoul",
    "Asia/Shanghai",
    "Asia/Hong_Kong",
    "Asia/Taipei",
    "Asia/Singapore",
    "Asia/Kuala_Lumpur",
    "Asia/Bangkok",
    "Asia/Ho_Chi_Minh",
    "Asia/Jakarta",
    "Asia/Makassar",
    "Asia/Manila",
    "Asia/Kolkata",
    "Asia/Dubai",
    "Europe/London",
    "Europe/Dublin",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Rome",
    "Europe/Madrid",
    "Europe/Amsterdam",
    "Europe/Zurich",
    "Europe/Vienna",
    "Europe/Prague",
    "Europe/Athens",
    "Europe/Istanbul",
    "Atlantic/Reykjavik",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Phoenix",
    "America/Los_Angeles",
    "America/Anchorage",
    "America/Vancouver",
    "America/Toronto",
    "Pacific/Honolulu",
    "UTC",
];

/// How an item's zone was found.
#[derive(Debug, Clone, PartialEq)]
pub enum How {
    /// The item has its own.
    Set,
    /// Worked out from the journeys around it.
    WorkedOut,
    /// The time happens in more than one place on that day; this is a guess.
    Ambiguous,
}

#[derive(Debug, Clone)]
pub struct Placed {
    pub zone: TimeZone,
    /// The zone's name, such as "Asia/Tokyo".
    pub name: String,
    pub how: How,
}

/// A stretch of the trip spent in one zone: from arriving (or the start of
/// time) to the next departure (or the end of time).
#[derive(Debug, Clone)]
struct Stretch {
    zone: TimeZone,
    name: String,
    from: Option<Timestamp>,
    to: Option<Timestamp>,
}

impl Stretch {
    fn contains(&self, t: Timestamp) -> bool {
        self.from.is_none_or(|f| t >= f) && self.to.is_none_or(|e| t < e)
    }
}

/// The trip's stretches, worked out once and asked many times.
#[derive(Debug, Clone)]
pub struct Timeline {
    home: TimeZone,
    home_name: String,
    stretches: Vec<Stretch>,
}

fn name_of(z: &TimeZone, fallback: &str) -> String {
    z.iana_name().unwrap_or(fallback).to_string()
}

fn at(w: &When, z: &TimeZone) -> Option<Timestamp> {
    w.civil().to_zoned(z.clone()).ok().map(|z| z.timestamp())
}

impl Trip {
    /// The zone at home: the trip's, or this computer's.
    pub fn home_zone(&self) -> TimeZone {
        if !self.home_tz.trim().is_empty() {
            if let Ok(z) = TimeZone::get(self.home_tz.trim()) {
                return z;
            }
        }
        TimeZone::system()
    }

    pub fn timeline(&self) -> Timeline {
        let home = self.home_zone();
        let home_name = name_of(&home, "home");
        // Journeys that say where they arrive, in order of arrival.
        let mut journeys: Vec<(&Item, TimeZone, Timestamp)> = self
            .items
            .iter()
            .filter(|i| i.kind == Kind::Travel && i.status != Status::Cancelled)
            .filter_map(|i| {
                let e = i.end.as_ref()?;
                let z = e.zone()?;
                let t = at(e, &z)?;
                Some((i, z, t))
            })
            .collect();
        journeys.sort_by_key(|j| j.2);

        let mut stretches = vec![Stretch {
            zone: home.clone(),
            name: home_name.clone(),
            from: None,
            to: None,
        }];
        for (item, arrive_zone, arrive) in journeys {
            let here = stretches.last().unwrap().zone.clone();
            // Leaving: in the journey's own zone, or wherever you were.
            let leave = item
                .start
                .as_ref()
                .and_then(|s| at(s, &s.zone().unwrap_or(here)))
                .unwrap_or(arrive);
            stretches.last_mut().unwrap().to = Some(leave.min(arrive));
            stretches.push(Stretch {
                name: name_of(&arrive_zone, "?"),
                zone: arrive_zone,
                from: Some(arrive),
                to: None,
            });
        }
        Timeline {
            home,
            home_name,
            stretches,
        }
    }
}

impl Timeline {
    pub fn home(&self) -> &TimeZone {
        &self.home
    }

    pub fn home_name(&self) -> &str {
        &self.home_name
    }

    /// The zone a day and time is in.
    pub fn place(&self, w: &When) -> Placed {
        self.place_leaning(w, None)
    }

    /// The zone the start (or end) of an item of `kind` is in. When the time
    /// happens in two places on a travel day, a stay's check-in is taken to
    /// be after arriving and its check-out before leaving, so those aren't
    /// asked about.
    pub fn place_as(&self, w: &When, kind: Kind, end: bool) -> Placed {
        let lean = match (kind, end) {
            (Kind::Stay, false) => Some(true),
            (Kind::Stay, true) => Some(false),
            _ => None,
        };
        self.place_leaning(w, lean)
    }

    /// Where an item's start (or end) happens.
    pub fn place_in(&self, item: &Item, end: bool) -> Option<Placed> {
        let w = if end {
            item.end.as_ref()
        } else {
            item.start.as_ref()
        }?;
        Some(self.place_as(w, item.kind, end))
    }

    /// The moment an item starts (or ends).
    pub fn instant_in(&self, item: &Item, end: bool) -> Option<Timestamp> {
        let w = if end {
            item.end.as_ref()
        } else {
            item.start.as_ref()
        }?;
        at(w, &self.place_as(w, item.kind, end).zone)
    }

    /// `lean`: when two places fit, Some(true) takes the later, Some(false)
    /// the earlier, and None takes the later but marks it ambiguous.
    fn place_leaning(&self, w: &When, lean: Option<bool>) -> Placed {
        if let Some(z) = w.zone() {
            return Placed {
                name: w.tz.trim().to_string(),
                zone: z,
                how: How::Set,
            };
        }
        // An untimed item: wherever you are at midday.
        let probe = if w.time.is_some() {
            w.clone()
        } else {
            When::at(w.date, jiff::civil::time(12, 0, 0, 0))
        };
        let fits: Vec<&Stretch> = self
            .stretches
            .iter()
            .filter(|s| at(&probe, &s.zone).is_some_and(|t| s.contains(t)))
            .collect();
        let placed = |s: &Stretch, how| Placed {
            zone: s.zone.clone(),
            name: s.name.clone(),
            how,
        };
        match fits.as_slice() {
            [one] => placed(one, How::WorkedOut),
            [first, ..] => {
                // Two places at once: the later one unless told otherwise,
                // and say so (unless there's no time, when it hardly
                // matters).
                let last = fits.last().unwrap_or(first);
                match lean {
                    Some(false) => placed(first, How::WorkedOut),
                    Some(true) => placed(last, How::WorkedOut),
                    None => {
                        let how = if w.time.is_some()
                            && fits.iter().any(|s| !same_offset(s, last, &probe))
                        {
                            How::Ambiguous
                        } else {
                            How::WorkedOut
                        };
                        placed(last, how)
                    }
                }
            }
            [] => {
                // In between (on a plane, or the dates don't line up): the
                // last place you'd arrived in by then, by the home clock.
                let t = at(&probe, &self.home);
                let s = self
                    .stretches
                    .iter()
                    .rev()
                    .find(|s| match (s.from, t) {
                        (Some(f), Some(t)) => f <= t,
                        _ => true,
                    })
                    .unwrap_or(&self.stretches[0]);
                placed(s, How::WorkedOut)
            }
        }
    }

    /// The moment a day and time happens, in its own zone.
    pub fn instant(&self, w: &When) -> Option<Timestamp> {
        at(w, &self.place(w).zone)
    }

    /// The same moment on the home clock, when it isn't the same as the
    /// local one.
    pub fn home_time(&self, w: &When) -> Option<DateTime> {
        w.time?;
        let local = self.place(w).zone;
        let t = at(w, &local)?;
        let home = t.to_zoned(self.home.clone());
        (home.offset() != t.to_zoned(local).offset()).then(|| home.datetime())
    }

    /// What time an item happens at home, when that isn't the local time:
    /// "Sun 08:45", or for a journey "Sat 09:45 to Sat 18:35".
    pub fn home_times(&self, item: &Item) -> Option<String> {
        let start = item.start.as_ref().filter(|w| w.time.is_some());
        let end = item
            .end
            .as_ref()
            .filter(|w| w.time.is_some() && item.kind == Kind::Travel);
        if start.and_then(|w| self.home_time(w)).is_none()
            && end.and_then(|w| self.home_time(w)).is_none()
        {
            return None;
        }
        let fmt = |w: &When| {
            at(w, &self.place(w).zone).map(|t| {
                t.to_zoned(self.home.clone())
                    .strftime("%a %H:%M")
                    .to_string()
            })
        };
        match (start.and_then(fmt), end.and_then(fmt)) {
            (Some(a), Some(b)) => Some(format!("{a} to {b}")),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(format!("arrives {b}")),
            (None, None) => None,
        }
    }

    /// Sort items by when they start: by day, untimed first on a day, then
    /// by the moment they happen.
    pub fn sort(&self, v: &mut [&Item]) {
        v.sort_by_cached_key(|i| {
            let s = i.start.as_ref();
            (
                s.is_none(),
                s.map(|w| w.date),
                s.is_some_and(|w| w.time.is_some()),
                s.and_then(|w| w.time.and_then(|_| self.instant_in(i, false))),
                i.display_title(),
            )
        });
    }
}

fn same_offset(a: &Stretch, b: &Stretch, w: &When) -> bool {
    match (at(w, &a.zone), at(w, &b.zone)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

impl Trip {
    /// Items starting on `day`, in the order they happen.
    pub fn items_on(&self, day: Date) -> Vec<&Item> {
        let mut v: Vec<&Item> = self
            .items
            .iter()
            .filter(|i| i.date() == Some(day))
            .collect();
        self.timeline().sort(&mut v);
        v
    }

    /// Journeys under way through the night of `day`: they leave on or
    /// before that day and arrive on a later one. A flight across the date
    /// line westward can cover two nights.
    pub fn travelling_overnight(&self, day: Date) -> Vec<&Item> {
        self.items
            .iter()
            .filter(|i| {
                i.kind == Kind::Travel
                    && i.status != Status::Cancelled
                    && i.date().is_some_and(|d| d <= day)
                    && i.end.as_ref().is_some_and(|e| e.date > day)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::{date, time};

    fn flight(from: (Date, Time, &str), to: (Date, Time, &str)) -> Item {
        let mut f = Item::new(Kind::Travel);
        f.status = Status::Booked;
        f.title = format!("{} to {}", from.2, to.2);
        f.start = Some(When::at(from.0, from.1).in_tz(from.2));
        f.end = Some(When::at(to.0, to.1).in_tz(to.2));
        f
    }
    use jiff::civil::Time;

    fn thing(title: &str, d: Date, t: Option<Time>) -> Item {
        let mut i = Item::new(Kind::Activity);
        i.title = title.into();
        i.start = Some(When {
            date: d,
            time: t,
            tz: String::new(),
        });
        i
    }

    /// Auckland to Los Angeles: leaves Tuesday 19:00, lands Tuesday 10:30.
    fn eastbound() -> Trip {
        let mut t = Trip::new("T");
        t.home_tz = "Australia/Sydney".into();
        let tue = date(2027, 3, 16);
        t.items.push(flight(
            (date(2027, 3, 10), time(9, 0, 0, 0), "Australia/Sydney"),
            (date(2027, 3, 10), time(14, 0, 0, 0), "Pacific/Auckland"),
        ));
        t.items.push(flight(
            (tue, time(19, 0, 0, 0), "Pacific/Auckland"),
            (tue, time(10, 30, 0, 0), "America/Los_Angeles"),
        ));
        t
    }

    #[test]
    fn a_flight_leaving_in_the_evening_sorts_before_the_afternoon_after_landing() {
        let mut t = eastbound();
        let tue = date(2027, 3, 16);
        let mut hotel = Item::new(Kind::Stay);
        hotel.title = "LA hotel".into();
        hotel.start = Some(When::at(tue, time(15, 0, 0, 0)).in_tz("America/Los_Angeles"));
        t.items.push(hotel);
        t.items
            .push(thing("Breakfast in Auckland", tue, Some(time(8, 0, 0, 0))));
        let order: Vec<String> = t.items_on(tue).iter().map(|i| i.display_title()).collect();
        assert_eq!(
            order,
            [
                "Breakfast in Auckland",
                "Pacific/Auckland to America/Los_Angeles",
                "LA hotel"
            ]
        );
    }

    #[test]
    fn works_out_the_zone_from_the_journeys() {
        let t = eastbound();
        let tl = t.timeline();
        // At home before the first flight, in Auckland after it.
        let p = tl.place(&When::at(date(2027, 3, 9), time(10, 0, 0, 0)));
        assert_eq!(
            (p.name.as_str(), p.how),
            ("Australia/Sydney", How::WorkedOut)
        );
        let p = tl.place(&When::at(date(2027, 3, 12), time(10, 0, 0, 0)));
        assert_eq!(p.name, "Pacific/Auckland");
        // In Los Angeles the day after landing.
        let p = tl.place(&When::at(date(2027, 3, 17), time(10, 0, 0, 0)));
        assert_eq!(p.name, "America/Los_Angeles");
        // Tuesday 12:00 happens in Auckland before the flight and in LA after.
        let p = tl.place(&When::at(date(2027, 3, 16), time(12, 0, 0, 0)));
        assert_eq!(p.how, How::Ambiguous);
        // Tuesday 06:00 is only in Auckland (LA isn't reached until 10:30).
        let p = tl.place(&When::at(date(2027, 3, 16), time(6, 0, 0, 0)));
        assert_eq!(
            (p.name.as_str(), p.how),
            ("Pacific/Auckland", How::WorkedOut)
        );
    }

    #[test]
    fn home_time_shown_only_when_it_differs() {
        let t = eastbound();
        let tl = t.timeline();
        // 10:30 in LA on 16 March is 04:30 on the 17th in Sydney (UTC+11).
        let landing = t.items[1].end.clone().unwrap();
        assert_eq!(
            tl.home_time(&landing),
            Some(date(2027, 3, 17).at(4, 30, 0, 0))
        );
        let at_home = When::at(date(2027, 3, 9), time(10, 0, 0, 0));
        assert_eq!(tl.home_time(&at_home), None);
    }

    #[test]
    fn a_check_in_on_a_travel_day_is_where_you_land() {
        let mut t = eastbound();
        let tue = date(2027, 3, 16);
        let mut hotel = Item::new(Kind::Stay);
        hotel.start = Some(When::at(tue, time(15, 0, 0, 0)));
        let tl = t.timeline();
        assert_eq!(tl.place(hotel.start.as_ref().unwrap()).how, How::Ambiguous);
        let p = tl.place_in(&hotel, false).unwrap();
        assert_eq!(
            (p.name.as_str(), p.how),
            ("America/Los_Angeles", How::WorkedOut)
        );
        t.items.push(hotel);
        assert!(crate::checks::check(&t, date(2027, 1, 1))
            .iter()
            .all(|w| !w.text.starts_with("Set a time zone")));
    }

    #[test]
    fn home_times_for_a_journey() {
        let t = eastbound();
        let tl = t.timeline();
        // Auckland 19:00 Tue is Sydney 17:00 Tue; LA 10:30 Tue is Sydney 04:30 Wed.
        assert_eq!(
            tl.home_times(&t.items[1]).unwrap(),
            "Tue 17:00 to Wed 04:30"
        );
    }

    #[test]
    fn westbound_over_the_date_line_covers_two_nights() {
        let mut t = Trip::new("T");
        // Los Angeles Monday 22:00 to Auckland Wednesday 06:00.
        t.items.push(flight(
            (date(2027, 3, 22), time(22, 0, 0, 0), "America/Los_Angeles"),
            (date(2027, 3, 24), time(6, 0, 0, 0), "Pacific/Auckland"),
        ));
        assert_eq!(t.travelling_overnight(date(2027, 3, 22)).len(), 1);
        assert_eq!(t.travelling_overnight(date(2027, 3, 23)).len(), 1);
        assert!(t.travelling_overnight(date(2027, 3, 24)).is_empty());
    }
}
