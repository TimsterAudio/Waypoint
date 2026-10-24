//! Airports, airlines and towns, built into the program so they work with no
//! internet: enough to turn "MEL" or "Auckland" into a time zone, and "NZ"
//! into Air New Zealand.
//!
//! The lists are in `data/` (made by `data/build_places.py`):
//! - airports and airlines from OpenFlights (openflights.org/data, Open
//!   Database License);
//! - towns from Natural Earth's populated places (public domain), as packed
//!   by the city-timezones package (MIT).

use std::sync::OnceLock;

const AIRPORTS: &str = include_str!("../data/airports.tsv");
const AIRLINES: &str = include_str!("../data/airlines.tsv");
const CITIES: &str = include_str!("../data/cities.tsv");

#[derive(Debug)]
pub struct Airport {
    pub code: &'static str,
    pub name: &'static str,
    pub city: &'static str,
    pub country: &'static str,
    pub tz: &'static str,
}

#[derive(Debug)]
pub struct City {
    pub name: &'static str,
    pub ascii: &'static str,
    pub country: &'static str,
    pub country_code: &'static str,
    pub province: &'static str,
    pub population: u64,
    pub tz: &'static str,
}

fn rows(text: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split('\t').collect())
}

fn airports() -> &'static [Airport] {
    static LIST: OnceLock<Vec<Airport>> = OnceLock::new();
    LIST.get_or_init(|| {
        rows(AIRPORTS)
            .filter(|r| r.len() >= 5)
            .map(|r| Airport {
                code: r[0],
                name: r[1],
                city: r[2],
                country: r[3],
                tz: r[4],
            })
            .collect()
    })
}

fn cities() -> &'static [City] {
    static LIST: OnceLock<Vec<City>> = OnceLock::new();
    LIST.get_or_init(|| {
        rows(CITIES)
            .filter(|r| r.len() >= 7)
            .map(|r| City {
                name: r[0],
                ascii: r[1],
                country: r[2],
                country_code: r[3],
                province: r[4],
                population: r[5].parse().unwrap_or(0),
                tz: r[6],
            })
            .collect()
    })
}

/// An airport by its three-letter code.
pub fn airport(code: &str) -> Option<&'static Airport> {
    let code = code.trim();
    if code.len() != 3 {
        return None;
    }
    let list = airports();
    let code = code.to_uppercase();
    list.binary_search_by(|a| a.code.cmp(code.as_str()))
        .ok()
        .map(|i| &list[i])
}

/// An airline's name by its two-character code: "NZ" is Air New Zealand.
pub fn airline(code: &str) -> Option<&'static str> {
    static MAP: OnceLock<Vec<(&'static str, &'static str)>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        rows(AIRLINES)
            .filter(|r| r.len() >= 2)
            .map(|r| (r[0], r[1]))
            .collect()
    });
    let code = code.trim().to_uppercase();
    if code.len() != 2 {
        return None;
    }
    map.binary_search_by(|(c, _)| c.cmp(&code.as_str()))
        .ok()
        .map(|i| map[i].1)
}

/// A flight number's airline code: "NZ120" gives "NZ", "UA 2830" gives "UA".
pub fn airline_code_of(number: &str) -> Option<&str> {
    let n = number.trim();
    let code = n.get(..2)?;
    let rest = n[2..].trim_start();
    (code.chars().all(|c| c.is_ascii_alphanumeric())
        && code.chars().any(|c| c.is_ascii_alphabetic())
        && !rest.is_empty()
        && rest.chars().all(|c| c.is_ascii_digit()))
    .then_some(code)
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// A three-letter code typed alone or in brackets: "MEL", "Melbourne (MEL)".
fn code_in(text: &str) -> Option<&str> {
    let t = text.trim();
    if t.len() == 3 && t.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(t);
    }
    let open = t.rfind('(')?;
    let close = t[open..].find(')')? + open;
    let inner = t[open + 1..close].trim();
    (inner.len() == 3 && inner.chars().all(|c| c.is_ascii_alphabetic())).then_some(inner)
}

/// A town by name, with an optional hint after a comma ("Melbourne, FL",
/// "Melbourne, Australia"); the largest wins.
pub fn city(text: &str) -> Option<&'static City> {
    let t = text.trim();
    let (name, hint) = match t.split_once(',') {
        Some((n, h)) => (n.trim(), Some(h.trim())),
        None => (t, None),
    };
    if name.is_empty() {
        return None;
    }
    cities()
        .iter()
        .filter(|c| same(c.name, name) || same(c.ascii, name))
        .filter(|c| {
            hint.is_none_or(|h| {
                h.is_empty()
                    || same(c.country, h)
                    || same(c.country_code, h)
                    || same(c.province, h)
                    || c.country.to_lowercase().starts_with(&h.to_lowercase())
                    || (h.len() == 2 && c.country_code == "US" && us_state(h) == Some(c.province))
            })
        })
        .max_by_key(|c| c.population)
}

fn us_state(code: &str) -> Option<&'static str> {
    const STATES: &[(&str, &str)] = &[
        ("AL", "Alabama"),
        ("AK", "Alaska"),
        ("AZ", "Arizona"),
        ("AR", "Arkansas"),
        ("CA", "California"),
        ("CO", "Colorado"),
        ("CT", "Connecticut"),
        ("DE", "Delaware"),
        ("FL", "Florida"),
        ("GA", "Georgia"),
        ("HI", "Hawaii"),
        ("ID", "Idaho"),
        ("IL", "Illinois"),
        ("IN", "Indiana"),
        ("IA", "Iowa"),
        ("KS", "Kansas"),
        ("KY", "Kentucky"),
        ("LA", "Louisiana"),
        ("ME", "Maine"),
        ("MD", "Maryland"),
        ("MA", "Massachusetts"),
        ("MI", "Michigan"),
        ("MN", "Minnesota"),
        ("MS", "Mississippi"),
        ("MO", "Missouri"),
        ("MT", "Montana"),
        ("NE", "Nebraska"),
        ("NV", "Nevada"),
        ("NH", "New Hampshire"),
        ("NJ", "New Jersey"),
        ("NM", "New Mexico"),
        ("NY", "New York"),
        ("NC", "North Carolina"),
        ("ND", "North Dakota"),
        ("OH", "Ohio"),
        ("OK", "Oklahoma"),
        ("OR", "Oregon"),
        ("PA", "Pennsylvania"),
        ("RI", "Rhode Island"),
        ("SC", "South Carolina"),
        ("SD", "South Dakota"),
        ("TN", "Tennessee"),
        ("TX", "Texas"),
        ("UT", "Utah"),
        ("VT", "Vermont"),
        ("VA", "Virginia"),
        ("WA", "Washington"),
        ("WV", "West Virginia"),
        ("WI", "Wisconsin"),
        ("WY", "Wyoming"),
        ("DC", "District of Columbia"),
    ];
    STATES
        .iter()
        .find(|(c, _)| c.eq_ignore_ascii_case(code))
        .map(|(_, n)| *n)
}

/// A place's time zone, worked out from what was typed for it: an airport
/// code ("MEL", "Sydney (SYD)"), a town ("Kyoto", "Melbourne, FL"), or an
/// airport's own name or town ("Haneda", "Dallas-Fort Worth").
pub fn zone_for(text: &str) -> Option<&'static str> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(a) = code_in(t).and_then(airport) {
        return Some(a.tz);
    }
    // Leave off a bracketed part ("Tokyo Haneda (HND)" without a known code).
    let bare = t.split('(').next().unwrap_or(t).trim();
    if let Some(c) = city(bare) {
        return Some(c.tz);
    }
    let lower = bare.to_lowercase();
    if lower.len() < 4 {
        return None;
    }
    airports()
        .iter()
        .find(|a| same(a.city, bare))
        .or_else(|| {
            airports()
                .iter()
                .find(|a| a.name.to_lowercase().contains(&lower))
        })
        .map(|a| a.tz)
}

/// A tidier name for a bare airport code: "MEL" becomes "Melbourne (MEL)".
/// Anything else is left as it is.
pub fn tidy_airport(text: &str) -> Option<String> {
    let t = text.trim();
    if t.len() != 3 || !t.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let a = airport(t)?;
    Some(format!("{} ({})", a.city, a.code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_load_and_are_sorted() {
        assert!(airports().len() > 5000);
        assert!(cities().len() > 7000);
        assert!(airports().windows(2).all(|w| w[0].code < w[1].code));
        // Every zone in the lists is one the program knows.
        for z in airports()
            .iter()
            .map(|a| a.tz)
            .chain(cities().iter().map(|c| c.tz))
        {
            assert!(jiff::tz::TimeZone::get(z).is_ok(), "unknown zone {z}");
        }
    }

    #[test]
    fn zones_from_places() {
        assert_eq!(zone_for("MEL"), Some("Australia/Melbourne"));
        assert_eq!(zone_for("mel"), Some("Australia/Melbourne"));
        assert_eq!(zone_for("Melbourne"), Some("Australia/Melbourne"));
        assert_eq!(zone_for("Melbourne, FL"), Some("America/New_York"));
        assert_eq!(zone_for("Sydney (SYD)"), Some("Australia/Sydney"));
        assert_eq!(zone_for("Auckland"), Some("Pacific/Auckland"));
        assert_eq!(zone_for("Houston"), Some("America/Chicago"));
        assert_eq!(zone_for("Dallas"), Some("America/Chicago"));
        assert_eq!(zone_for("Kyoto"), Some("Asia/Tokyo"));
        assert_eq!(zone_for("Tokyo Haneda (HND)"), Some("Asia/Tokyo"));
        assert_eq!(zone_for("Los Angeles"), Some("America/Los_Angeles"));
        assert_eq!(zone_for("Nowhere-on-Sea"), None);
        assert_eq!(zone_for(""), None);
    }

    #[test]
    fn airlines_and_codes() {
        assert_eq!(airline("NZ"), Some("Air New Zealand"));
        assert_eq!(airline("qf"), Some("Qantas"));
        assert_eq!(airline("UA"), Some("United Airlines"));
        assert_eq!(airline_code_of("NZ120"), Some("NZ"));
        assert_eq!(airline_code_of("UA 2830"), Some("UA"));
        assert_eq!(airline_code_of("120"), None);
        assert_eq!(airline_code_of("Nozomi 215"), None);
        assert_eq!(tidy_airport("akl").unwrap(), "Auckland (AKL)");
        assert_eq!(tidy_airport("Auckland"), None);
    }
}
