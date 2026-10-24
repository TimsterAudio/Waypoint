//! Places as you type: hotels, attractions and addresses from Photon
//! (photon.komoot.io), a free search over OpenStreetMap that allows
//! searching as you type and needs no account. Only used when switched on in
//! Settings. Blocking: run it off the UI thread.

use std::time::Duration;

use serde::Deserialize;

/// A place found: its name, an address to show and store, and the time zone
/// of the nearest town (a good guess near borders).
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub name: String,
    pub address: String,
    /// What it is, from OpenStreetMap: "hotel", "museum", "restaurant".
    pub what: String,
    pub tz: Option<&'static str>,
}

#[derive(Deserialize)]
struct Collection {
    #[serde(default)]
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    #[serde(default)]
    properties: Props,
    geometry: Option<Geometry>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Props {
    name: Option<String>,
    housenumber: Option<String>,
    street: Option<String>,
    locality: Option<String>,
    district: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postcode: Option<String>,
    country: Option<String>,
    osm_value: Option<String>,
}

#[derive(Deserialize)]
struct Geometry {
    coordinates: Vec<f64>,
}

/// Photon's answer, as places. Ones with no name and no street are left out.
pub fn parse_photon(json: &str) -> Result<Vec<Found>, String> {
    let c: Collection = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out: Vec<Found> = Vec::new();
    for f in c.features {
        let p = f.properties;
        let street = match (&p.housenumber, &p.street) {
            (Some(n), Some(s)) => Some(format!("{n} {s}")),
            (None, Some(s)) => Some(s.clone()),
            _ => None,
        };
        let town = p.city.clone().or(p.locality.clone()).or(p.district.clone());
        let state_post = match (&p.state, &p.postcode) {
            (Some(s), Some(pc)) => Some(format!("{s} {pc}")),
            (Some(s), None) => Some(s.clone()),
            (None, Some(pc)) => Some(pc.clone()),
            _ => None,
        };
        let address: Vec<String> = [street.clone(), town.clone(), state_post, p.country.clone()]
            .into_iter()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .collect();
        let name = p.name.clone().or(street).or(town).unwrap_or_default();
        if name.trim().is_empty() {
            continue;
        }
        let tz = f
            .geometry
            .filter(|g| g.coordinates.len() >= 2)
            .and_then(|g| crate::places::nearest_city(g.coordinates[1], g.coordinates[0]))
            .map(|c| c.tz);
        let found = Found {
            name: name.trim().to_string(),
            address: address.join(", "),
            what: p.osm_value.unwrap_or_default().replace('_', " "),
            tz,
        };
        if !out
            .iter()
            .any(|o| o.name == found.name && o.address == found.address)
        {
            out.push(found);
        }
    }
    Ok(out)
}

/// Search for `text`. Returns up to six places, or why not.
pub fn search(text: &str) -> Result<Vec<Found>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!(
            "Waypoint/",
            env!("CARGO_PKG_VERSION"),
            " (holiday planner; github.com/TimsterAudio/Waypoint)"
        ))
        .build()
        .map_err(|e| e.to_string())?;
    let r = client
        .get("https://photon.komoot.io/api/")
        .query(&[("q", text), ("limit", "6"), ("lang", "en")])
        .send()
        .map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("the search service said {}", r.status()));
    }
    parse_photon(&r.text().map_err(|e| e.to_string())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of Photon's answer (GeoJSON), cut down.
    const SAMPLE: &str = r#"{"type":"FeatureCollection","features":[
      {"type":"Feature","geometry":{"type":"Point","coordinates":[144.885,-37.708]},
       "properties":{"osm_type":"N","osm_key":"tourism","osm_value":"hotel","name":"Quest Tullamarine",
         "housenumber":"20","street":"Annandale Road","district":"Tullamarine","city":"Melbourne",
         "state":"Victoria","postcode":"3045","country":"Australia","type":"house"}},
      {"type":"Feature","geometry":{"type":"Point","coordinates":[-96.826,32.951]},
       "properties":{"osm_value":"hotel","name":"La Quinta Inn","street":"Landmark Boulevard",
         "city":"Addison","state":"Texas","country":"United States"}},
      {"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{}}
    ]}"#;

    #[test]
    fn reads_photon() {
        let f = parse_photon(SAMPLE).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].name, "Quest Tullamarine");
        assert_eq!(
            f[0].address,
            "20 Annandale Road, Melbourne, Victoria 3045, Australia"
        );
        assert_eq!(f[0].what, "hotel");
        assert_eq!(f[0].tz, Some("Australia/Melbourne"));
        assert_eq!(
            f[1].address,
            "Landmark Boulevard, Addison, Texas, United States"
        );
        assert_eq!(f[1].tz, Some("America/Chicago"));
        assert!(parse_photon("not json").is_err());
    }
}
