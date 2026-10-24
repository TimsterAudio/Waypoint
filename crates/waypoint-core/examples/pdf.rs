//! Writes the sample trip's PDFs, to check how they look:
//! `cargo run -p waypoint-core --example pdf -- OUT_FOLDER`.

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let trip = waypoint_core::sample::trip();
    let now = jiff::Zoned::now();
    for (shared, name) in [(false, "sample.pdf"), (true, "sample (to share).pdf")] {
        let pdf = waypoint_core::pdf::render(&trip, &now, shared).expect("the PDF renders");
        std::fs::write(dir.join(name), pdf).expect("can write the PDF");
        println!("Wrote {name}");
    }
}
