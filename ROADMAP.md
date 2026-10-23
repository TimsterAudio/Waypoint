# Roadmap

Nothing here is promised; it's the order things seem worth doing in.

## Next

- **Try it for real:** a trip in Google Drive on Windows and Linux, and the phone page on the iPhone from the Files app. Fix what that turns up.
- **Calendar export:** an `.ics` file beside the trip, so bookings show in the phone's calendar.
- **Print / PDF** of the itinerary.
- **Attachments:** link confirmation PDFs and tickets (kept in the same Drive folder) to an item, and link them from the phone page.

## After the first runs

Asked for on 2026-10-23, to add once 0.1 has been tried.

- **Pet care while away:** where the pets are boarded (or who's minding them), drop-off and pick-up dates and times, contact details, cost, status and notes. Covers dates like a stay does, and the Overview should flag days the trip covers with no care booked.
- **Travel insurance:** insurer, policy number, who's covered, cover dates, the 24-hour emergency assistance number, claims contact, excess, cost and notes. Shown near the top of the phone itinerary, since it's what's needed in an emergency. The Overview should flag trip days outside the cover dates.
- **Passports and visas:** for each traveller, passport number, issuing country and expiry, flagged when it expires within six months of getting home; each visa or e-travel authorisation (Australian ETA, US ESTA, UK ETA and so on) with its number, the countries it covers, valid dates, entries allowed and cost. Visas due to be applied for show on the Overview.

## To review

Ideas to look at after the first runs, then schedule or drop.

From other travel apps:

- **Before you go:** a to-do list with due dates (foreign cash, phone roaming or eSIM, mail hold, home security, parking at the airport).
- **Packing list,** reusable from one trip to the next.
- **Spending on the trip:** actual daily spending against a daily budget, separate from bookings.
- **Emergency and local information:** embassy, local emergency number, power plugs, tipping, time difference from home.
- **Loyalty numbers:** frequent flyer and hotel programmes per traveller.
- **Baggage allowance, seat and meal** on each flight.
- **Copy a past trip** as the start of a new one.

Typing less:

- **Places as you type:** suggest hotels and attractions with their address while typing a name. Needs an online place search: Photon (komoot, OpenStreetMap data) allows search as you type, free and with no key; OpenStreetMap's own Nominatim forbids it; Google Places needs a billing account. Off unless switched on, like exchange rates.
- **Airlines and airports from a list built in:** type "QF" or "Qantas", "SYD" or "Sydney" and get the name, code and city, and the airport's time zone filled in. No internet needed (OpenFlights' airline and airport lists). Looking up a flight number's times needs a paid flight-data service, so that would be a later, optional step.
- **Quicker dates:** choosing a day in the calendar closes it straight away (or on a double-click) instead of needing Save, and a date can be typed ("14/5").

## Later

- **A live phone page:** a web app (GitHub Pages, added to the iPhone's home screen) that signs in to Google and reads and edits the trip file in Drive, using Drive access limited to the files Waypoint made. Needs a Google Cloud project, set up once.
- **Direct Google Drive on Linux** through that same Google project, so rclone isn't needed.
- **Travellers and split costs:** who each item is for, and who owes whom.
- **Checklists:** packing, documents, things to do before going.
- **Import from a spreadsheet** (CSV), for trips planned the old way.
- **Merging** two people's changes item by item instead of choosing one version.

## Parked

- Maps (needs map tiles from the internet).
- Reading booking details from confirmation emails.
