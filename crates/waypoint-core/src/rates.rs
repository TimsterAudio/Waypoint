//! Exchange rates from two free services that need no account or key.
//!
//! - **Frankfurter** (api.frankfurter.dev): the European Central Bank's
//!   reference rates, about 30 currencies, updated each working day.
//! - **ExchangeRate-API's open access** (open.er-api.com): about 160
//!   currencies, updated daily. Used for any currency Frankfurter lacks.
//!
//! Rates are only fetched when asked for. Blocking: run it off the UI thread.

use std::collections::BTreeMap;
use std::time::Duration;

use jiff::civil::Date;
use serde::Deserialize;

use crate::model::Rate;

pub const FRANKFURTER: &str = "Frankfurter (European Central Bank)";
pub const OPEN_ER: &str = "ExchangeRate-API";

#[derive(Deserialize)]
struct Frankfurter {
    date: Date,
    rates: BTreeMap<String, f64>,
}

/// Frankfurter's answer for `base`: units of each currency per one `base`,
/// turned round into `base` per unit.
pub fn parse_frankfurter(json: &str) -> Result<BTreeMap<String, Rate>, String> {
    let f: Frankfurter = serde_json::from_str(json).map_err(|e| e.to_string())?;
    Ok(invert(f.rates, Some(f.date), FRANKFURTER))
}

#[derive(Deserialize)]
struct OpenEr {
    result: String,
    #[serde(default)]
    time_last_update_unix: Option<i64>,
    #[serde(default)]
    rates: BTreeMap<String, f64>,
}

pub fn parse_open_er(json: &str) -> Result<BTreeMap<String, Rate>, String> {
    let r: OpenEr = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if r.result != "success" {
        return Err(format!("the service said \"{}\"", r.result));
    }
    let day = r
        .time_last_update_unix
        .and_then(|s| jiff::Timestamp::from_second(s).ok())
        .map(|t| t.to_zoned(jiff::tz::TimeZone::UTC).date());
    Ok(invert(r.rates, day, OPEN_ER))
}

fn invert(
    per_base: BTreeMap<String, f64>,
    as_of: Option<Date>,
    source: &str,
) -> BTreeMap<String, Rate> {
    per_base
        .into_iter()
        .filter(|(_, v)| *v > 0.0)
        .map(|(code, v)| {
            (
                code,
                Rate {
                    home_per_unit: 1.0 / v,
                    as_of,
                    source: source.into(),
                },
            )
        })
        .collect()
}

/// Today's rates into `home` for each of `wanted`. Returns what it found and
/// a message for anything it couldn't.
pub fn fetch(home: &str, wanted: &[String]) -> (BTreeMap<String, Rate>, Option<String>) {
    let mut found = BTreeMap::new();
    if wanted.is_empty() {
        return (found, None);
    }
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("Waypoint/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => return (found, Some(format!("Couldn't start a connection: {e}"))),
    };
    let get = |url: &str| -> Result<String, String> {
        let r = client.get(url).send().map_err(|e| e.to_string())?;
        if !r.status().is_success() {
            return Err(format!("HTTP {}", r.status()));
        }
        r.text().map_err(|e| e.to_string())
    };
    let mut errors = Vec::new();

    let url = format!(
        "https://api.frankfurter.dev/v1/latest?base={home}&symbols={}",
        wanted.join(",")
    );
    match get(&url).and_then(|j| parse_frankfurter(&j)) {
        Ok(r) => found.extend(r.into_iter().filter(|(c, _)| wanted.contains(c))),
        Err(e) => errors.push(format!("Frankfurter: {e}")),
    }

    let rest: Vec<&String> = wanted.iter().filter(|c| !found.contains_key(*c)).collect();
    if !rest.is_empty() {
        match get(&format!("https://open.er-api.com/v6/latest/{home}"))
            .and_then(|j| parse_open_er(&j))
        {
            Ok(r) => found.extend(r.into_iter().filter(|(c, _)| rest.contains(&c))),
            Err(e) => errors.push(format!("ExchangeRate-API: {e}")),
        }
    }

    let missing: Vec<&String> = wanted.iter().filter(|c| !found.contains_key(*c)).collect();
    let message = if missing.is_empty() {
        None
    } else if !errors.is_empty() && found.is_empty() {
        Some(format!(
            "Couldn't get exchange rates. {}",
            errors.join("; ")
        ))
    } else {
        Some(format!(
            "No rate found for {}. You can type one in.",
            missing
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    };
    (found, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_frankfurter() {
        let json =
            r#"{"amount":1.0,"base":"AUD","date":"2026-10-22","rates":{"EUR":0.5912,"JPY":98.4}}"#;
        let r = parse_frankfurter(json).unwrap();
        assert!((r["JPY"].home_per_unit - 1.0 / 98.4).abs() < 1e-12);
        assert_eq!(r["EUR"].as_of, Some(jiff::civil::date(2026, 10, 22)));
        assert_eq!(r["EUR"].source, FRANKFURTER);
    }

    #[test]
    fn reads_open_er() {
        let json = r#"{"result":"success","provider":"https://www.exchangerate-api.com",
            "time_last_update_unix":1792540801,"base_code":"AUD",
            "rates":{"AUD":1,"VND":16650.2,"FJD":1.47}}"#;
        let r = parse_open_er(json).unwrap();
        assert!((r["VND"].home_per_unit - 1.0 / 16650.2).abs() < 1e-15);
        assert!(r["VND"].as_of.is_some());
        assert!(parse_open_er(r#"{"result":"error"}"#).is_err());
    }
}
