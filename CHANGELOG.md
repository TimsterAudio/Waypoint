# Changelog

## Unreleased

- **Time zones across the date line** (issues 1–3 on GitHub).
  - Items are put in the order they actually happen, not by their local clock: a flight leaving Auckland on Tuesday evening now comes before the Los Angeles hotel check-in that Tuesday afternoon. Items without their own time zone take it from the journeys around them (at home before the first, then wherever each one lands).
  - A flight covering two nights (westward over the date line) marks both as "travelling overnight", in the app, the checks and the phone itinerary.
  - Bookings in another time zone also show the time at home ("home Tue 17:00 to Wed 04:30"), in the itinerary and on the phone itinerary. The trip has a home time zone (Trip details), which new trips take from the computer.
  - When a time without a zone happens twice on one day (eastward over the date line), Waypoint asks for a zone instead of guessing silently. A stay's check-in is taken to be where you land and its check-out where you leave, so those aren't asked about.
  - The time zone box shows the zone it worked out.
- **Item editor:** a long message (such as the one asking for a time zone) no longer widens the panel and pushes the field names off its left edge (issue 5).
- **Itinerary rows:** the home time has its own line and shows the end time too ("Home time: Sat 16:00 to Wed 12:00"); stays show their address and phone (issues 4 and 6).
- **Itinerary to share** (issue 7): a page for family and friends with what's booked, where and when, how to reach each place and home times, and nothing private (no costs, booking references, who it's booked with, contacts, seats, rooms, notes, ideas or cancellations). Written beside the trip as "Name (to share).html" from the Itinerary or Trip details, and opened in the browser; print it there and choose Save as PDF for a PDF.

## 0.1.0 – 2026-10-23

The first version.

- Trips as single `.trip.json` files, meant for a Google Drive folder. Saves by itself; notices changes saved on another computer and takes them in, or asks whose to keep when both changed; says when someone else has the trip open.
- Items of five kinds (stay, travel, activity, meal, other) with status (idea, booked, confirmed, paid, cancelled), start and end with optional time and time zone, place and address, who it's booked with, reference, contact, phone, email, website, cost and notes. Travel adds mode, carrier, number, from, to, terminal and seats; stays add the room.
- Costs in any currency with the home currency (AUD by default) worked out from the trip's exchange rates; marking an item paid locks its rate. Rates fetched from Frankfurter and ExchangeRate-API when asked, or typed in.
- Screens: Overview (countdown, money, budget, things to look at, what's next), Itinerary (day by day, with tonight's stay), Bookings (filterable table), Costs (budget, by kind, still to pay, rates), Trip details, Settings.
- Checks: nights with nowhere to stay or only ideas, double bookings, overlapping bookings, balances due within 14 days, free cancellation ending within 7 days, missing references and dates, items outside the trip.
- The phone itinerary: a self-contained page written beside the trip on each save, for the iPhone's Files app.
- A sample trip (`--sample`).
