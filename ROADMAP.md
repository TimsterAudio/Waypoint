# Waypoint roadmap

Updates and add-ons, from the owner and from Claude. Problems go in GitHub issues (the Gap form); this list is for new things. The owner reviews it on a printable checklist page (a claude.ai artifact, "Waypoint roadmap review") with the same IDs, and marks each item **Yes** (build it), *Later* (keep, not now) or no (drop it). Items with no mark are still undecided; don't build them.

Tags: **Offline** works with no internet, **Online** uses a free online service (off until switched on, like exchange rates), **Paid** needs a paid service. *Asked* marks the owner's own ideas.

Status: `[ ]` open, `[x]` done, `[-]` dropped, `[~]` later.

## Decisions so far (2026-10-24)

- **Yes:** A1, A2, A3 (asked for, to add after the first runs)
- **Undecided:** everything else

## Next, whatever is decided

- Try it for real on a trip: Google Drive on Windows and Linux, the phone page on the iPhone. Problems found go in GitHub issues.

## A. Asked for after the first runs
- [ ] **A1** Pet care while away: where the pets are boarded or who's minding them, drop-off and pick-up dates and times, contact, cost, status, notes. Covers dates like a stay; the Overview flags trip days with no care booked. *Offline, Asked* — **Yes**
- [ ] **A2** Travel insurance: insurer, policy number, who's covered, cover dates, the 24-hour emergency number, claims contact, excess, cost, notes. Near the top of the phone itinerary; the Overview flags trip days outside the cover. *Offline, Asked* — **Yes**
- [ ] **A3** Passports and visas: each traveller's passport number, country and expiry (flagged when it expires within six months of getting home); each visa or e-travel authorisation (ETA, ESTA and so on) with number, countries, valid dates, entries and cost. Visas still to apply for show on the Overview. *Offline, Asked* — **Yes**

## B. Typing less
- [ ] **B1** Time zones filled in from where a journey leaves and arrives: type "MEL", "LAX" or "Auckland" in From and To and both zones are set. Airports from OpenFlights' list; stations, ports and towns from GeoNames' list of towns over 15,000 people (about 2 MB). A zone set by hand is never overwritten. *Offline, Asked*
- [ ] **B2** Airline names from their codes: "NZ" becomes Air New Zealand (OpenFlights' airline list). *Offline*
- [ ] **B3** Places as you type: hotels and attractions with their address while typing a name, from Photon (OpenStreetMap data; free, no key, allows search as you type). A place picked this way could set its time zone too. *Online, Asked*
- [ ] **B4** Quicker dates: picking a day closes the calendar straight away (or on a double-click) instead of needing Save, and dates can be typed ("14/5"). *Offline, Asked*

## C. Flights
- [ ] **C1** Flight times while planning: a flight number and date give the scheduled times, airports and terminal, from a schedule service such as AeroDataBox (check it reaches far enough ahead). *Paid, Asked*
- [ ] **C2** Live flight status on the day: actual departure, delays and landing, from the Flightradar24 API (its own paid plans, separate from a Silver or Gold account; flights in the air and already flown, not future schedules) or similar. Belongs with the live phone page (F1). *Paid, Asked*
- [ ] **C3** Baggage allowance, seat and meal on each flight. *Offline*

## D. Before and during the trip
- [ ] **D1** Before you go: a to-do list with due dates (foreign cash, eSIM or roaming, mail hold, home security, airport parking). *Offline*
- [ ] **D2** Packing list, reusable from trip to trip. *Offline*
- [ ] **D3** Spending on the trip: what's actually spent each day against a daily budget, separate from bookings. *Offline*
- [ ] **D4** Emergency and local information: embassy, local emergency number, power plugs, tipping, time difference from home. *Offline*
- [ ] **D5** Loyalty numbers: frequent flyer and hotel programmes for each traveller. *Offline*

## E. Sharing and printing
- [ ] **E1** Calendar export: an `.ics` file beside the trip so bookings show in the phone's calendar. *Offline*
- [ ] **E2** Attachments: link tickets and confirmation PDFs (kept in the same Drive folder) to an item, and link them from the phone page. *Offline*
- [ ] **E3** Save straight to PDF from Waypoint, without going through a browser. *Offline*
- [ ] **E4** Choose notes to show on the itinerary to share (all notes are left off now). *Offline*

## F. Bigger features
- [ ] **F1** A live phone page: a web app (GitHub Pages, on the iPhone's home screen) that signs in to Google and reads and edits the trip in Drive, with access only to files Waypoint made. Needs a Google Cloud project, set up once. *Online*
- [ ] **F2** Google Drive directly on Linux, through that same project, so rclone isn't needed. *Online*
- [ ] **F3** Travellers and split costs: who each item is for, and who owes whom. *Offline*
- [ ] **F4** Import from a spreadsheet (CSV), for trips planned the old way. *Offline*
- [ ] **F5** Merge two people's changes item by item instead of choosing one version. *Offline*
- [ ] **F6** Copy a past trip as the start of a new one. *Offline*

## G. Parked
- [ ] **G1** Maps (needs map tiles from the internet). *Online*
- [ ] **G2** Reading booking details from confirmation emails. *Online*

## Done

Built from the owner's notes while planning the first trip (GitHub issues 2, 4, 6 and 7).

- [x] **Z1** Home time on bookings in other time zones, with start and end, on its own line in the itinerary and on the phone page. *Asked* — done 2026-10-23
- [x] **Z2** A home time zone for the trip (Trip details), taken from the computer for new trips. — done 2026-10-23
- [x] **Z3** Address and phone for stays on the itinerary. *Asked* — done 2026-10-23
- [x] **Z4** Itinerary to share: booked items without costs, references or notes, printable to PDF from the browser. *Asked* — done 2026-10-23
