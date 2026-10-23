# Waypoint: context for Claude Code

A holiday planner, started on 2026-10-23 to replace the owner's holiday spreadsheet ("a mess of sheets that is organised in my head more than the data") without paying for a subscription app. The README covers what it does; read it too.

## What the owner wants

- Windows and Linux, the look of their Isobar app (TimsterAudio/Isobar), one program with nothing else to install.
- Trips stored in **Google Drive** (they have a Google account). On the go they'll use an **iPhone**, through an HTML page linked to the Drive file.
- Costs paid finally in **AUD**; the travel spots use other currencies.
- Someone else **may** edit the same trip.
- The minimum features they listed: cloud storage, currency rates, day-by-day activities with times, references, contacts and locations, day-by-day accommodation, daily travel (flights, trains), Idea / Booked / Confirmed / Paid, costs on everything, notes, dates and times.

## Decisions already made (don't revisit without asking)

- **Stack:** pure Rust, eframe/egui 0.36, like Isobar. `crates/waypoint-core` holds everything without a window and is unit tested; `src/` is the window.
- **Look:** Isobar's: `src/theme.rs` and most of `src/ui.rs` are copied from it, with the Barlow fonts. Violet (`trace`) marks travel, today and the selected item. Kinds have a colour each (`Palette::kind`).
- **Storage:** one JSON file per trip, `Name.trip.json`, in a folder Google Drive syncs; Waypoint doesn't talk to Google itself (yet). JSON rather than SQLite so a phone page can read it later and a person can too. Saves write a temporary file and rename it. Every struct has `#[serde(default)]`, and `Trip` and `Item` keep unknown fields (`extra`) so different versions don't drop each other's data. `format` is the file version; a newer one refuses to open.
- **Two editors:** a `.trip.lock` beside the trip names who has it open (stale after 12 h). The file's fingerprint is checked before each save and every 3 s: changes from elsewhere load by themselves when nothing is unsaved; otherwise the person chooses "load theirs, keep mine as a copy" or "save mine over theirs". Never overwrite silently.
- **Autosave** 1.5 s after the last edit. Edits are found by comparing the trip before and after each frame (`Doc::edited`), which also stamps `updated` and `updatedBy` on changed items.
- **Phone itinerary:** a self-contained HTML page (`html.rs`) with no scripts and nothing fetched, written beside the trip on every save. Taken from the Files app on iOS. A live web app that signs in to Drive is the planned next stage (see ROADMAP.md).
- **Money:** amounts kept in the currency paid; converted with the trip's rates; **marking Paid locks the rate** on the item. Rates from Frankfurter, then ExchangeRate-API's open endpoint for the rest; only fetched when asked or when the setting is on. No network otherwise.
- **Time zones** (`zones.rs`): an item's own zone if set; otherwise worked out from journeys with an arrival zone (home before the first, then each arrival's zone until the next departure). Items sort by the moment they happen within each local day. A time that fits two stretches (eastward over the date line) is ambiguous and asked about, except a stay's check-in (taken as after landing) and check-out (before leaving). The trip's `homeTz` (empty means the computer's) drives "home" times. Travel from day d to a later day covers every night in between.
- **Itinerary to share** (`html::render_shared`): built from a copy of the trip with private fields removed and only booked items kept, so nothing private can leak through the page code. Printing from the browser is the PDF route; no PDF library.
- **Copy style:** sentence case, plain words, specific messages. Day-first dates.

## Checks before pushing

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo clippy --target x86_64-pc-windows-gnu -- -D warnings
```

For anything visible, `cargo build` and run `./target/debug/waypoint --sample` under Xvfb (needs `libxkbcommon-x11-0`; `xdotool` to click, and `xdotool windowfocus` before typing since there's no window manager), in light and dark, and look. Clippy and tests don't rebuild the binary you run. The phone page: `cargo run -p waypoint-core --example phone_page -- out.html`, then look at it at iPhone width (Playwright's iPhone 13 device with `/opt/pw-browsers` Chromium). The sandbox can't reach the rate services, so `rates.rs` is tested against saved responses.

## State

0.1.0 built and checked in a Linux cloud session on 2026-10-23; every screen looked at in light and dark with the sample; a real trip file opened, edited, autosaved with its phone page; a change saved on disk by "someone else" taken in; a clash stopped and resolved with a copy; the Windows `.exe` cross-compiles. Since then (Unreleased): time zones across the date line, from the owner's first gaps (GitHub issues 1–3), checked with an Auckland–Los Angeles round trip; then issues 4–7 (editor panel widening, itinerary row lines, the itinerary to share). The owner reports gaps as GitHub issues with the Gap form, titled "GAP: …".
