# Changelog

## Unreleased

## 0.1.0 – 2026-10-23

The first version.

- Trips as single `.trip.json` files, meant for a Google Drive folder. Saves by itself; notices changes saved on another computer and takes them in, or asks whose to keep when both changed; says when someone else has the trip open.
- Items of five kinds (stay, travel, activity, meal, other) with status (idea, booked, confirmed, paid, cancelled), start and end with optional time and time zone, place and address, who it's booked with, reference, contact, phone, email, website, cost and notes. Travel adds mode, carrier, number, from, to, terminal and seats; stays add the room.
- Costs in any currency with the home currency (AUD by default) worked out from the trip's exchange rates; marking an item paid locks its rate. Rates fetched from Frankfurter and ExchangeRate-API when asked, or typed in.
- Screens: Overview (countdown, money, budget, things to look at, what's next), Itinerary (day by day, with tonight's stay), Bookings (filterable table), Costs (budget, by kind, still to pay, rates), Trip details, Settings.
- Checks: nights with nowhere to stay or only ideas, double bookings, overlapping bookings, balances due within 14 days, free cancellation ending within 7 days, missing references and dates, items outside the trip.
- The phone itinerary: a self-contained page written beside the trip on each save, for the iPhone's Files app.
- A sample trip (`--sample`).
