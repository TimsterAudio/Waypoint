//! Writes the sample trip's phone itinerary, to check how it looks:
//! `cargo run -p waypoint-core --example phone_page -- sample.html`.

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "sample.html".into());
    let trip = waypoint_core::sample::trip();
    // As it would look on the second day of the trip.
    let now = jiff::civil::date(2027, 5, 16)
        .at(8, 0, 0, 0)
        .in_tz("Asia/Tokyo")
        .expect("Tokyo is a known zone");
    std::fs::write(&out, waypoint_core::html::render(&trip, now)).expect("can write the page");
    println!("Wrote {out}");
}
