//! A trip and everything in it. This is what a `.trip.json` file holds.
//!
//! Every field has a default, so a file from an older version loads, and
//! fields this version doesn't know are kept (`extra`) and written back, so
//! two people on different versions don't lose each other's data.

use std::collections::BTreeMap;

use jiff::civil::{Date, DateTime, Time};
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

/// The file format version this build writes.
pub const FORMAT: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Trip {
    pub format: u32,
    pub name: String,
    /// The currency the costs are finally paid in, such as "AUD".
    pub home_currency: String,
    /// The time zone at home, such as "Australia/Sydney", for showing what
    /// time it is at home. Empty means this computer's zone.
    pub home_tz: String,
    pub start: Option<Date>,
    pub end: Option<Date>,
    /// In the home currency.
    pub budget: Option<f64>,
    pub travellers: Vec<String>,
    /// Exchange rates into the home currency, by currency code.
    pub rates: BTreeMap<String, Rate>,
    pub items: Vec<Item>,
    pub notes: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for Trip {
    fn default() -> Self {
        Trip {
            format: FORMAT,
            name: String::new(),
            home_currency: "AUD".into(),
            home_tz: String::new(),
            start: None,
            end: None,
            budget: None,
            travellers: Vec::new(),
            rates: BTreeMap::new(),
            items: Vec::new(),
            notes: String::new(),
            extra: Default::default(),
        }
    }
}

/// How much one unit of a currency is worth in the home currency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Rate {
    pub home_per_unit: f64,
    /// The day the rate is for.
    pub as_of: Option<Date>,
    /// Where it came from: a service's name, or "Entered by hand".
    pub source: String,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Stay,
    Travel,
    #[default]
    Activity,
    Meal,
    Other,
}

impl Kind {
    pub const ALL: [Kind; 5] = [
        Kind::Stay,
        Kind::Travel,
        Kind::Activity,
        Kind::Meal,
        Kind::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Stay => "Stay",
            Kind::Travel => "Travel",
            Kind::Activity => "Activity",
            Kind::Meal => "Meal",
            Kind::Other => "Other",
        }
    }
}

/// How far along a booking is. Cancelled items stay in the file, so a
/// cancellation fee or lost deposit still counts.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Idea,
    Booked,
    Confirmed,
    Paid,
    Cancelled,
}

impl Status {
    pub const ALL: [Status; 5] = [
        Status::Idea,
        Status::Booked,
        Status::Confirmed,
        Status::Paid,
        Status::Cancelled,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Status::Idea => "Idea",
            Status::Booked => "Booked",
            Status::Confirmed => "Confirmed",
            Status::Paid => "Paid",
            Status::Cancelled => "Cancelled",
        }
    }

    /// Booked, confirmed or paid: money that will be spent.
    pub fn committed(self) -> bool {
        matches!(self, Status::Booked | Status::Confirmed | Status::Paid)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Flight,
    Train,
    Bus,
    Ferry,
    Car,
    Transfer,
    Other,
}

impl Mode {
    pub const ALL: [Mode; 7] = [
        Mode::Flight,
        Mode::Train,
        Mode::Bus,
        Mode::Ferry,
        Mode::Car,
        Mode::Transfer,
        Mode::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Flight => "Flight",
            Mode::Train => "Train",
            Mode::Bus => "Bus",
            Mode::Ferry => "Ferry",
            Mode::Car => "Car hire",
            Mode::Transfer => "Transfer",
            Mode::Other => "Other",
        }
    }
}

/// A day, an optional time on it, and the time zone it's in. An empty zone
/// means "local time wherever this is", which is fine for anything that
/// doesn't cross a border.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct When {
    pub date: Date,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<Time>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tz: String,
}

impl When {
    pub fn day(date: Date) -> When {
        When {
            date,
            time: None,
            tz: String::new(),
        }
    }

    pub fn at(date: Date, time: Time) -> When {
        When {
            date,
            time: Some(time),
            tz: String::new(),
        }
    }

    pub fn in_tz(mut self, tz: &str) -> When {
        self.tz = tz.into();
        self
    }

    /// The local date and time, midnight when there's no time.
    pub fn civil(&self) -> DateTime {
        self.date.to_datetime(self.time.unwrap_or(Time::midnight()))
    }

    /// The zone, if one is set and known.
    pub fn zone(&self) -> Option<TimeZone> {
        if self.tz.is_empty() {
            None
        } else {
            TimeZone::get(&self.tz).ok()
        }
    }

    /// Seconds since 1970 in UTC, when the zone is known.
    pub fn instant(&self) -> Option<i64> {
        let tz = self.zone()?;
        self.civil()
            .to_zoned(tz)
            .ok()
            .map(|z| z.timestamp().as_second())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Cost {
    /// A currency code, such as "JPY". Empty means the trip's home currency.
    pub currency: String,
    pub amount: f64,
    /// How much of `amount` has been paid so far.
    pub paid: f64,
    /// When the rest is due.
    pub due: Option<Date>,
    /// The last day it can be cancelled free.
    pub cancel_by: Option<Date>,
    /// The rate it was paid at, home currency per unit. Set when it's
    /// marked paid, so the total stops moving with the exchange rate.
    pub locked_rate: Option<f64>,
}

impl Cost {
    pub fn is_empty(&self) -> bool {
        self.amount == 0.0 && self.paid == 0.0
    }

    pub fn owing(&self) -> f64 {
        (self.amount - self.paid).max(0.0)
    }
}

/// Extra details for travel: a flight, train, ferry, car hire and so on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Travel {
    pub mode: Mode,
    /// The airline, rail company or hire firm.
    pub carrier: String,
    /// Flight or train number.
    pub number: String,
    pub from: String,
    pub to: String,
    pub terminal: String,
    pub seat: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Item {
    pub id: String,
    pub kind: Kind,
    pub status: Status,
    pub title: String,
    /// When it starts. For a stay, check-in; for travel, departure.
    pub start: Option<When>,
    /// When it ends. For a stay, check-out; for travel, arrival.
    pub end: Option<When>,
    /// A place name, such as "Hotel Gracery Shinjuku".
    pub place: String,
    pub address: String,
    /// Who it's booked with: a hotel, an agent, a tour company.
    pub provider: String,
    /// Booking reference or confirmation number.
    pub reference: String,
    pub contact: String,
    pub phone: String,
    pub email: String,
    pub website: String,
    pub cost: Cost,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub travel: Option<Travel>,
    /// The room, for a stay.
    pub room: String,
    pub notes: String,
    /// When it was last changed, and on which computer.
    pub updated: Option<jiff::Timestamp>,
    pub updated_by: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for Item {
    fn default() -> Self {
        Item {
            id: new_id(),
            kind: Kind::Activity,
            status: Status::Idea,
            title: String::new(),
            start: None,
            end: None,
            place: String::new(),
            address: String::new(),
            provider: String::new(),
            reference: String::new(),
            contact: String::new(),
            phone: String::new(),
            email: String::new(),
            website: String::new(),
            cost: Cost::default(),
            travel: None,
            room: String::new(),
            notes: String::new(),
            updated: None,
            updated_by: String::new(),
            extra: Default::default(),
        }
    }
}

impl Item {
    pub fn new(kind: Kind) -> Item {
        Item {
            kind,
            travel: (kind == Kind::Travel).then(Travel::default),
            ..Default::default()
        }
    }

    /// The day it starts on, if it has one.
    pub fn date(&self) -> Option<Date> {
        self.start.as_ref().map(|w| w.date)
    }

    /// A title to show even when none was typed: "QF 25 Sydney to Tokyo".
    pub fn display_title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.trim().to_string();
        }
        if let Some(t) = &self.travel {
            let mut s = format!("{} {}", t.carrier, t.number).trim().to_string();
            if !t.from.is_empty() || !t.to.is_empty() {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(&format!("{} to {}", t.from, t.to));
            }
            if !s.is_empty() {
                return s;
            }
        }
        if !self.place.trim().is_empty() {
            return self.place.trim().to_string();
        }
        format!("Untitled {}", self.kind.label().to_lowercase())
    }

    /// The nights a stay covers: check-in up to the night before check-out.
    /// A stay with no check-out covers its first night.
    pub fn nights(&self) -> Vec<Date> {
        if self.kind != Kind::Stay {
            return Vec::new();
        }
        let Some(first) = self.date() else {
            return Vec::new();
        };
        let last = match &self.end {
            Some(e) if e.date > first => e.date.yesterday().unwrap_or(first),
            _ => first,
        };
        let mut out = Vec::new();
        let mut d = first;
        while d <= last && out.len() < 366 {
            out.push(d);
            match d.tomorrow() {
                Ok(n) => d = n,
                Err(_) => break,
            }
        }
        out
    }

    /// How long it lasts, in minutes, when both ends have a time and the
    /// zones (if any) are known. Crossing zones is allowed for: a flight
    /// leaving Sydney at 10:00 and landing in Tokyo at 18:30 is 9½ hours.
    pub fn duration_minutes(&self) -> Option<i64> {
        let (s, e) = (self.start.as_ref()?, self.end.as_ref()?);
        s.time?;
        e.time?;
        let secs = match (s.instant(), e.instant()) {
            (Some(a), Some(b)) => b - a,
            _ => {
                let a = s.civil();
                let b = e.civil();
                a.until(b).ok()?.total(jiff::Unit::Second).ok()? as i64
            }
        };
        Some(secs / 60)
    }
}

impl Trip {
    pub fn new(name: &str) -> Trip {
        Trip {
            name: name.into(),
            home_tz: TimeZone::system()
                .iana_name()
                .unwrap_or_default()
                .to_string(),
            ..Default::default()
        }
    }

    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn item_mut(&mut self, id: &str) -> Option<&mut Item> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Every day from the trip's start to its end, or from the earliest to
    /// the latest dated item when the trip has no dates.
    pub fn days(&self) -> Vec<Date> {
        let dated = self
            .items
            .iter()
            .flat_map(|i| [i.start.as_ref(), i.end.as_ref()])
            .flatten()
            .map(|w| w.date);
        let first = self.start.or_else(|| dated.clone().min());
        let last = self.end.or_else(|| dated.max());
        let (Some(first), Some(last)) = (first, last) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut d = first;
        while d <= last && out.len() < 1000 {
            out.push(d);
            match d.tomorrow() {
                Ok(n) => d = n,
                Err(_) => break,
            }
        }
        out
    }

    /// The stays covering the night of `day`, cancelled ones left out.
    pub fn stays_for_night(&self, day: Date) -> Vec<&Item> {
        self.items
            .iter()
            .filter(|i| i.status != Status::Cancelled && i.nights().contains(&day))
            .collect()
    }

    /// Items with no date yet, most often ideas.
    pub fn undated(&self) -> Vec<&Item> {
        self.items.iter().filter(|i| i.start.is_none()).collect()
    }

    /// Every currency used by an item other than the home currency.
    pub fn foreign_currencies(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .items
            .iter()
            .map(|i| i.cost.currency.trim().to_uppercase())
            .filter(|c| !c.is_empty() && *c != self.home_currency)
            .collect();
        v.sort();
        v.dedup();
        v
    }
}

/// A new item id: unique enough across two people's computers.
pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let n = COUNT.fetch_add(1, Ordering::Relaxed);
    let mix =
        nanos ^ (std::process::id() as u64).rotate_left(40) ^ n.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    format!("{:x}{:04x}", mix, n & 0xffff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::{date, time};

    #[test]
    fn stay_nights_stop_before_check_out() {
        let mut s = Item::new(Kind::Stay);
        s.start = Some(When::day(date(2027, 5, 14)));
        s.end = Some(When::day(date(2027, 5, 17)));
        assert_eq!(
            s.nights(),
            vec![date(2027, 5, 14), date(2027, 5, 15), date(2027, 5, 16)]
        );
        s.end = None;
        assert_eq!(s.nights(), vec![date(2027, 5, 14)]);
    }

    #[test]
    fn flight_duration_crosses_zones() {
        let mut f = Item::new(Kind::Travel);
        f.start = Some(When::at(date(2027, 5, 14), time(10, 0, 0, 0)).in_tz("Australia/Sydney"));
        f.end = Some(When::at(date(2027, 5, 14), time(18, 30, 0, 0)).in_tz("Asia/Tokyo"));
        // Sydney is UTC+10 in May, Tokyo UTC+9.
        assert_eq!(f.duration_minutes(), Some(9 * 60 + 30));
    }

    #[test]
    fn unknown_fields_survive_a_round_trip() {
        let json = r#"{"format":1,"name":"T","futureThing":{"a":1},
            "items":[{"id":"x","kind":"meal","later":true}]}"#;
        let t: Trip = serde_json::from_str(json).unwrap();
        assert_eq!(t.items[0].kind, Kind::Meal);
        let back = serde_json::to_value(&t).unwrap();
        assert_eq!(back["futureThing"]["a"], 1);
        assert_eq!(back["items"][0]["later"], true);
    }

    #[test]
    fn travel_title_from_details() {
        let mut f = Item::new(Kind::Travel);
        let t = f.travel.as_mut().unwrap();
        t.carrier = "QF".into();
        t.number = "25".into();
        t.from = "Sydney".into();
        t.to = "Tokyo".into();
        assert_eq!(f.display_title(), "QF 25 Sydney to Tokyo");
    }

    #[test]
    fn ids_differ() {
        assert_ne!(new_id(), new_id());
    }
}
