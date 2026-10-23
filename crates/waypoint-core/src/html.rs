//! The phone itinerary: one self-contained HTML page written beside the trip
//! file each time it's saved. No scripts and nothing fetched from the web,
//! so it shows in the iPhone's Files app and works with no signal.

use jiff::civil::Date;

use crate::checks::day_label;
use crate::model::{Item, Kind, Status, Trip};

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            _ => o.push(c),
        }
    }
    o
}

/// Percent-encode for a URL query.
fn url_enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                o.push(b as char)
            }
            b' ' => o.push('+'),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

fn time_of(w: &crate::model::When) -> String {
    w.time
        .map(|t| t.strftime("%H:%M").to_string())
        .unwrap_or_default()
}

fn status_class(s: Status) -> &'static str {
    match s {
        Status::Idea => "idea",
        Status::Booked => "booked",
        Status::Confirmed => "confirmed",
        Status::Paid => "paid",
        Status::Cancelled => "cancelled",
    }
}

const CSS: &str = r#"
:root{--paper:#e9eee8;--sheet:#f6f8f4;--grid:#cdd8cb;--faint:#dfe6dd;--ink:#1c2633;--ink2:#5a6570;--trace:#5b3a9e;--warn:#a8700c;--ok:#2f7a3e;--heat:#b3362b}
@media (prefers-color-scheme:dark){:root{--paper:#131a20;--sheet:#19222a;--grid:#2a3741;--faint:#222d36;--ink:#e6ebee;--ink2:#9aa7b2;--trace:#b49cf0;--warn:#e2b04e;--ok:#7fcf8b;--heat:#f0806c}}
*{box-sizing:border-box}
body{margin:0;background:var(--paper);color:var(--ink);font:16px/1.45 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;-webkit-text-size-adjust:100%}
main{max-width:720px;margin:0 auto;padding:20px 16px 48px}
h1{font-size:30px;line-height:1.1;margin:8px 0 4px;letter-spacing:-.01em}
.sub{color:var(--ink2);margin:0 0 16px}
nav{display:flex;flex-wrap:wrap;gap:6px;margin:0 0 20px}
nav a{font-size:13px;padding:4px 10px;border:1px solid var(--grid);border-radius:14px;color:var(--ink2);text-decoration:none}
.money{display:flex;gap:16px;flex-wrap:wrap;border-top:1px solid var(--grid);border-bottom:1px solid var(--grid);padding:10px 0;margin:0 0 24px}
.money div{min-width:120px}.money small{display:block;color:var(--ink2);font-size:13px}.money b{font-size:20px}
section.day{margin:0 0 22px}
section.day>h2{font-size:19px;margin:0 0 8px;position:sticky;top:0;background:var(--paper);padding:6px 0;border-bottom:2px solid var(--trace)}
.today>h2{color:var(--trace)}
.item{background:var(--sheet);border:1px solid var(--grid);border-radius:6px;padding:10px 12px;margin:0 0 8px}
.item.cancelled{opacity:.55}.item.cancelled .title{text-decoration:line-through}
.row{display:flex;gap:10px;align-items:baseline}
.time{font-variant-numeric:tabular-nums;font-weight:600;min-width:46px;color:var(--trace)}
.title{font-weight:600;flex:1}
.badge{font-size:12px;padding:1px 8px;border-radius:10px;border:1px solid var(--grid);color:var(--ink2);white-space:nowrap}
.badge.paid,.badge.confirmed{border-color:var(--ok);color:var(--ok)}
.badge.idea{border-style:dashed}
dl{display:grid;grid-template-columns:auto 1fr;gap:2px 12px;margin:6px 0 0 56px;font-size:14px}
dt{color:var(--ink2)}dd{margin:0;overflow-wrap:anywhere}
a{color:var(--trace)}
.ref{font-family:ui-monospace,Menlo,monospace;font-size:15px;letter-spacing:.03em}
.notes{margin:6px 0 0 56px;font-size:14px;white-space:pre-wrap;color:var(--ink2)}
.stay{font-size:14px;color:var(--ink2);margin:4px 0 0;padding-left:4px}
.stay b{color:var(--ink)}
.none{color:var(--heat)}
.owe{color:var(--warn)}
footer{color:var(--ink2);font-size:12px;margin-top:32px;border-top:1px solid var(--grid);padding-top:8px}
@media print{:root{--paper:#fff;--sheet:#fff;--ink:#000;--ink2:#444;--grid:#bbb;--trace:#4a2f86}nav{display:none}section.day>h2{position:static}.item,.stay{break-inside:avoid}section.day>h2{break-after:avoid}a{color:inherit;text-decoration:none}}
"#;

/// The whole page. `now` is shown as when it was made, and marks today.
pub fn render(trip: &Trip, now: jiff::Zoned) -> String {
    page(trip, now, false)
}

/// A page to share with family and friends: what's booked, where and when,
/// with how to reach each place, and nothing private: no costs, booking
/// references, who it's booked with, contacts, seats, rooms, notes, ideas
/// or cancellations. Prints well, for a PDF.
pub fn render_shared(trip: &Trip, now: jiff::Zoned) -> String {
    page(&shareable(trip), now, true)
}

/// A copy of the trip with the private details taken out, so nothing
/// private can reach the shared page by mistake.
fn shareable(trip: &Trip) -> Trip {
    let mut t = Trip {
        name: trip.name.clone(),
        home_currency: trip.home_currency.clone(),
        home_tz: trip.home_tz.clone(),
        start: trip.start,
        end: trip.end,
        travellers: trip.travellers.clone(),
        ..Default::default()
    };
    t.items = trip
        .items
        .iter()
        .filter(|i| i.status.committed())
        .map(|i| {
            let mut i = i.clone();
            i.cost = Default::default();
            i.reference.clear();
            i.provider.clear();
            i.contact.clear();
            i.email.clear();
            i.room.clear();
            i.notes.clear();
            i.updated_by.clear();
            i.extra.clear();
            if let Some(tr) = &mut i.travel {
                tr.seat.clear();
            }
            i
        })
        .collect();
    t
}

fn page(trip: &Trip, now: jiff::Zoned, shared: bool) -> String {
    let today = now.date();
    let mut h = String::new();
    h.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">");
    h.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    h.push_str(&format!(
        "<title>{}</title><style>{CSS}</style></head><body><main>",
        esc(&trip.name)
    ));
    h.push_str(&format!("<h1>{}</h1>", esc(&trip.name)));
    if let (Some(s), Some(e)) = (trip.start, trip.end) {
        let nights = s.until(e).map(|sp| sp.get_days()).unwrap_or(0);
        h.push_str(&format!(
            "<p class=\"sub\">{} to {} · {} nights{}</p>",
            day_label(s),
            day_label(e),
            nights,
            if shared && !trip.travellers.is_empty() {
                format!(" · {}", esc(&trip.travellers.join(", ")))
            } else {
                String::new()
            }
        ));
    }
    if !shared {
        money(&mut h, trip);
    }

    let days = trip.days();
    if days.len() > 1 {
        h.push_str("<nav>");
        for d in &days {
            h.push_str(&format!("<a href=\"#d{d}\">{}</a>", d.strftime("%a %-d")));
        }
        if !trip.undated().is_empty() {
            h.push_str("<a href=\"#undated\">No date</a>");
        }
        h.push_str("</nav>");
    }

    for d in &days {
        day(&mut h, trip, *d, *d == today, shared);
    }

    let undated = trip.undated();
    if !undated.is_empty() {
        h.push_str("<section class=\"day\" id=\"undated\"><h2>No date yet</h2>");
        for i in undated {
            item(&mut h, trip, &trip.timeline(), i, shared);
        }
        h.push_str("</section>");
    }

    if !trip.notes.trim().is_empty() {
        h.push_str(&format!(
            "<section class=\"day\"><h2>Notes</h2><div class=\"item\"><div class=\"notes\" style=\"margin:0\">{}</div></div></section>",
            esc(&trip.notes)
        ));
    }

    let footer = if shared {
        format!(
            "Our plans as of {}. Times are local to each place; home times are {}.",
            today.strftime("%-d %B %Y"),
            trip.timeline().home_name()
        )
    } else {
        format!(
            "Made by Waypoint on {} at {}. Open Waypoint and save the trip to bring this up to date.",
            today.strftime("%a %-d %b %Y"),
            now.strftime("%H:%M")
        )
    };
    h.push_str(&format!("<footer>{}</footer>", esc(&footer)));
    h.push_str("</main></body></html>\n");
    h
}

/// Booked, paid and still to pay, across the top.
fn money(h: &mut String, trip: &Trip) {
    let t = trip.totals();
    let home = &trip.home_currency;
    h.push_str("<div class=\"money\">");
    h.push_str(&format!(
        "<div><small>Booked</small><b>{}</b></div>",
        crate::money::format(t.committed, home)
    ));
    h.push_str(&format!(
        "<div><small>Paid</small><b>{}</b></div>",
        crate::money::format(t.paid, home)
    ));
    h.push_str(&format!(
        "<div><small>Still to pay</small><b class=\"{}\">{}</b></div>",
        if t.owing() > 0.0 { "owe" } else { "" },
        crate::money::format(t.owing(), home)
    ));
    h.push_str("</div>");
}

fn day(h: &mut String, trip: &Trip, d: Date, today: bool, shared: bool) {
    h.push_str(&format!(
        "<section class=\"day{}\" id=\"d{d}\"><h2>{}{}</h2>",
        if today { " today" } else { "" },
        esc(&d.strftime("%A %-d %B").to_string()),
        if today { " · today" } else { "" }
    ));
    let tl = trip.timeline();
    for i in trip.items_on(d) {
        item(h, trip, &tl, i, shared);
    }
    // Where you sleep tonight.
    let last_night = trip.end.is_some_and(|e| d >= e);
    let stays = trip.stays_for_night(d);
    if !stays.is_empty() {
        let names: Vec<String> = stays
            .iter()
            .map(|s| format!("<b>{}</b>", esc(&s.display_title())))
            .collect();
        h.push_str(&format!(
            "<p class=\"stay\">Tonight: {}</p>",
            names.join(" or ")
        ));
    } else if !last_night && !trip.travelling_overnight(d).is_empty() {
        h.push_str("<p class=\"stay\">Tonight: travelling overnight</p>");
    } else if !shared && !last_night && trip.start.is_some_and(|s| d >= s) {
        h.push_str("<p class=\"stay none\">Tonight: nowhere booked yet</p>");
    }
    h.push_str("</section>");
}

fn item(h: &mut String, trip: &Trip, tl: &crate::zones::Timeline, i: &Item, shared: bool) {
    h.push_str(&format!("<div class=\"item {}\">", status_class(i.status)));
    let time = i.start.as_ref().map(time_of).unwrap_or_default();
    let badge = if shared {
        String::new()
    } else {
        format!(
            "<span class=\"badge {}\">{}</span>",
            status_class(i.status),
            i.status.label()
        )
    };
    h.push_str(&format!(
        "<div class=\"row\"><span class=\"time\">{}</span><span class=\"title\">{}</span>{badge}</div>",
        esc(&time),
        esc(&i.display_title()),
    ));
    let mut rows: Vec<(&str, String)> = Vec::new();
    match i.kind {
        Kind::Stay => {
            if let Some(s) = &i.start {
                let t = time_of(s);
                rows.push((
                    "Check in",
                    if t.is_empty() {
                        day_label(s.date)
                    } else {
                        format!("{} from {t}", day_label(s.date))
                    },
                ));
            }
            if let Some(e) = &i.end {
                let t = time_of(e);
                rows.push((
                    "Check out",
                    if t.is_empty() {
                        day_label(e.date)
                    } else {
                        format!("{} by {t}", day_label(e.date))
                    },
                ));
            }
            if !i.room.is_empty() {
                rows.push(("Room", esc(&i.room)));
            }
        }
        Kind::Travel => {
            if let Some(t) = &i.travel {
                let what = format!("{} {} {}", t.mode.label(), t.carrier, t.number);
                rows.push(("", esc(what.trim())));
                if !t.from.is_empty() {
                    rows.push(("From", esc(&t.from)));
                }
                if !t.terminal.is_empty() {
                    rows.push(("Terminal", esc(&t.terminal)));
                }
                if !t.to.is_empty() {
                    rows.push(("To", esc(&t.to)));
                }
                if !t.seat.is_empty() {
                    rows.push(("Seat", esc(&t.seat)));
                }
            }
            if let Some(e) = &i.end {
                let t = time_of(e);
                let same_day = i.date() == Some(e.date);
                let at = match (t.is_empty(), same_day) {
                    (true, _) => day_label(e.date),
                    (false, true) => t,
                    (false, false) => format!("{t}, {}", day_label(e.date)),
                };
                let mut at = at;
                if let Some(m) = i.duration_minutes() {
                    at.push_str(&format!(" ({}h {:02}m)", m / 60, m % 60));
                }
                rows.push(("Arrives", esc(&at)));
            }
        }
        _ => {
            if let Some(e) = &i.end {
                let t = time_of(e);
                if !t.is_empty() {
                    rows.push(("Until", t));
                }
            }
        }
    }
    if let Some(home) = tl.home_times(i) {
        rows.push(("Home time", esc(&home)));
    }
    if !i.reference.is_empty() {
        rows.push((
            "Ref",
            format!("<span class=\"ref\">{}</span>", esc(&i.reference)),
        ));
    }
    if !i.provider.is_empty() {
        rows.push(("With", esc(&i.provider)));
    }
    if !i.place.is_empty() && i.place != i.display_title() {
        rows.push(("Place", esc(&i.place)));
    }
    if !i.address.is_empty() {
        rows.push((
            "Address",
            format!(
                "<a href=\"https://maps.apple.com/?q={}\">{}</a>",
                url_enc(&i.address),
                esc(&i.address)
            ),
        ));
    }
    if !i.contact.is_empty() {
        rows.push(("Contact", esc(&i.contact)));
    }
    if !i.phone.is_empty() {
        let dial: String = i
            .phone
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '+')
            .collect();
        rows.push((
            "Phone",
            format!("<a href=\"tel:{dial}\">{}</a>", esc(&i.phone)),
        ));
    }
    if !i.email.is_empty() {
        rows.push((
            "Email",
            format!("<a href=\"mailto:{0}\">{0}</a>", esc(&i.email)),
        ));
    }
    if !i.website.is_empty() {
        let href = if i.website.starts_with("http") {
            i.website.clone()
        } else {
            format!("https://{}", i.website)
        };
        rows.push((
            "Web",
            format!("<a href=\"{}\">{}</a>", esc(&href), esc(&i.website)),
        ));
    }
    if !i.cost.is_empty() {
        let code = trip.currency_of(i);
        let mut s = crate::money::format(i.cost.amount, &code);
        if i.cost.owing() > 0.0 && i.status != Status::Cancelled {
            s.push_str(&format!(
                " · <span class=\"owe\">{} to pay{}</span>",
                crate::money::format(i.cost.owing(), &code),
                i.cost
                    .due
                    .map(|d| format!(" by {}", day_label(d)))
                    .unwrap_or_default()
            ));
        } else if i.cost.paid > 0.0 {
            s.push_str(" · paid");
        }
        rows.push(("Cost", s));
    }
    if !rows.is_empty() {
        h.push_str("<dl>");
        for (k, v) in rows {
            h.push_str(&format!("<dt>{k}</dt><dd>{v}</dd>"));
        }
        h.push_str("</dl>");
    }
    if !i.notes.trim().is_empty() {
        h.push_str(&format!(
            "<div class=\"notes\">{}</div>",
            esc(i.notes.trim())
        ));
    }
    h.push_str("</div>");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_links() {
        let mut t = crate::sample::trip();
        t.items[0].title = "<script>alert(1)</script>".into();
        let page = render(
            &t,
            jiff::civil::date(2027, 5, 15)
                .at(9, 0, 0, 0)
                .in_tz("Australia/Sydney")
                .unwrap(),
        );
        assert!(!page.contains("<script>"));
        assert!(page.contains("&lt;script&gt;"));
        assert!(page.contains("href=\"tel:+81"));
        assert!(page.contains("maps.apple.com/?q="));
        assert!(page.contains("class=\"day today\""));
        // The flight to Tokyo lands at 17:35 there, 18:35 in Sydney.
        assert!(
            page.contains("<dt>Home time</dt><dd>Sat 09:45 to Sat 18:35</dd>"),
            "{page}"
        );
    }

    #[test]
    fn the_shared_page_leaves_out_private_details() {
        let t = crate::sample::trip();
        let now = jiff::civil::date(2027, 3, 1)
            .at(9, 0, 0, 0)
            .in_tz("Australia/Sydney")
            .unwrap();
        let page = render_shared(&t, now);
        for private in [
            "QX7K2P", // a booking reference
            "¥",      // costs
            "$",
            "Day trip to Nikko", // an idea
            "Twin, non-smoking", // the room
            "41A",               // seats
            "Bare feet",         // item notes
            "Passports",         // trip notes
            "Booking site",      // who it's booked with
            "class=\"badge",     // statuses
            "nowhere booked",
        ] {
            assert!(!page.contains(private), "{private} is on the shared page");
        }
        for public in [
            "Hotel Kumo Shinjuku",
            "1-19-1 Kabukicho",
            "tel:+81312345678",
            "QF 25",
            "Home time",
        ] {
            assert!(
                page.contains(public),
                "{public} is missing from the shared page"
            );
        }
    }

    #[test]
    fn encodes_queries() {
        assert_eq!(url_enc("1-2 Shinjuku, Tokyo"), "1-2+Shinjuku%2C+Tokyo");
    }
}
