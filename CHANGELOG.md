# Changelog

## Unreleased

- **Who's going** takes names separated by commas again: a comma typed after a name was removed at once, so a second name couldn't be added.

- **A planning stage** (roadmap F7): a trip is Planning or Booking (Trip details; the Overview shows "Planning"). While it's planning and nothing is booked, confirmed or paid, changing the first day moves the whole trip and every idea in it, keeping its length and times; payment dates stay put. Once something is booked, changing the dates moves nothing. New trips start in Planning; trips saved before this open as Booking.

- **Mark things to look at as OK** (issues 9 and 11): each line under To look at has an OK link that stops it showing; "marked OK: show" lists them again, with Undo. The marks are saved in the trip, so they carry to the other computer.
- **No booking reference asked for** a drive in your own car, or a stay with no cost and nobody it's booked with (with family or friends) (issue 11).
- **Car**, for your own car, is a new way to travel, separate from Car hire (issue 8). A trip that uses it needs this version of Waypoint or later on every computer that opens it. A later version's new ways to travel read as Other here instead of stopping the trip opening.

- **Places as you type** (roadmap B3): with "Suggest places" switched on in Settings, typing a hotel, restaurant or place name lists matching places from OpenStreetMap (through Photon, free and with no account). Picking one fills in the name and address, and the time zone where the booking has none (from the nearest town). Off until switched on; nothing goes online otherwise.
- **Save PDF** (E3): the itinerary as a PDF made by Waypoint itself, either everything or the version to share, from the Itinerary (Save PDF) or Trip details. A4, in the app's fonts.

- **Time zones from places** (roadmap B1): a journey with no time zone set takes one from where it leaves and arrives, using lists built into Waypoint (about 5,500 airports and 7,300 towns), so "Melbourne", "MEL", "Houston (IAH)" or "Kyoto" are enough. The editor shows the zone each place gives, and the time zone box says "(from the place)". A zone you set yourself always wins. Typing an airport code in From or To becomes "Melbourne (MEL)" when you leave the box.
- **Flight lengths** use the zones of both ends, worked out or set, so Melbourne 08:15 to Auckland 13:55 is 3h 40m, not 5h 40m (issue 10).
- **Home times** more than a day apart carry their dates ("Fri 13 Nov 12:00 to Fri 20 Nov 03:00").
- **Airline names from codes** (B2): an airline typed as "NZ" becomes Air New Zealand, and a flight number such as "NZ120" fills in an empty airline.
- **Quicker dates** (B4): a date can be typed day first ("14/5", "14 May", "14/5/27"), and one click on a day in the new calendar picks it and closes it, with no Save.

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
