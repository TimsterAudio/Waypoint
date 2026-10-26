//! A made-up trip to look around with: ten days in Japan, partly booked,
//! with one night still to find. Names and numbers are invented.

use jiff::civil::{date, time, Date, Time};

use crate::model::{Item, Kind, Mode, Rate, Status, Travel, Trip, When};

fn at(d: Date, t: Time, tz: &str) -> Option<When> {
    Some(When::at(d, t).in_tz(tz))
}

fn yen(i: &mut Item, amount: f64, paid: f64) {
    i.cost.currency = "JPY".into();
    i.cost.amount = amount;
    i.cost.paid = paid;
}

pub fn trip() -> Trip {
    const SYD: &str = "Australia/Sydney";
    const TYO: &str = "Asia/Tokyo";
    let mut t = Trip::new("Japan 2027 (sample)");
    t.home_tz = SYD.into();
    t.planning = false;
    t.start = Some(date(2027, 5, 15));
    t.end = Some(date(2027, 5, 25));
    t.budget = Some(12_000.0);
    t.travellers = vec!["Tim".into(), "Sam".into()];
    t.rates.insert(
        "JPY".into(),
        Rate {
            home_per_unit: 0.0102,
            as_of: Some(date(2027, 3, 1)),
            source: "Entered by hand".into(),
        },
    );
    t.notes = "Passports expire 2031. Travel insurance: policy TI-0000-SAMPLE.".into();
    let mut items = Vec::new();

    let mut f = Item::new(Kind::Travel);
    f.status = Status::Paid;
    f.start = at(date(2027, 5, 15), time(9, 45, 0, 0), SYD);
    f.end = at(date(2027, 5, 15), time(17, 35, 0, 0), TYO);
    f.travel = Some(Travel {
        mode: Mode::Flight,
        carrier: "QF".into(),
        number: "25".into(),
        from: "Sydney (SYD)".into(),
        to: "Tokyo Haneda (HND)".into(),
        terminal: "T1 International".into(),
        seat: "41A, 41B".into(),
    });
    f.provider = "Qantas".into();
    f.reference = "QX7K2P".into();
    f.cost.amount = 3_680.0;
    f.cost.paid = 3_680.0;
    items.push(f);

    let mut h = Item::new(Kind::Stay);
    h.title = "Hotel Kumo Shinjuku".into();
    h.status = Status::Confirmed;
    h.start = at(date(2027, 5, 15), time(15, 0, 0, 0), TYO);
    h.end = at(date(2027, 5, 19), time(11, 0, 0, 0), TYO);
    h.address = "1-19-1 Kabukicho, Shinjuku, Tokyo".into();
    h.phone = "+81 3 1234 5678".into();
    h.provider = "Booking site".into();
    h.reference = "4417 902 318".into();
    h.room = "Twin, non-smoking".into();
    yen(&mut h, 92_000.0, 0.0);
    h.cost.due = Some(date(2027, 5, 1));
    h.cost.cancel_by = Some(date(2027, 5, 12));
    items.push(h);

    let mut a = Item::new(Kind::Activity);
    a.title = "Digital art museum".into();
    a.status = Status::Paid;
    a.start = at(date(2027, 5, 16), time(10, 0, 0, 0), TYO);
    a.end = at(date(2027, 5, 16), time(12, 0, 0, 0), TYO);
    a.place = "Toyosu".into();
    a.reference = "TL-55812".into();
    a.notes = "Bare feet inside; wear shorts or roll trousers up.".into();
    yen(&mut a, 7_600.0, 7_600.0);
    a.cost.locked_rate = Some(0.0104);
    items.push(a);

    let mut m = Item::new(Kind::Meal);
    m.title = "Sushi counter dinner".into();
    m.status = Status::Booked;
    m.start = at(date(2027, 5, 16), time(19, 0, 0, 0), TYO);
    m.end = at(date(2027, 5, 16), time(21, 0, 0, 0), TYO);
    m.place = "Ginza".into();
    m.contact = "Reservations desk".into();
    m.phone = "+81 3 8765 4321".into();
    m.reference = "Under 'Tim', 2 people".into();
    yen(&mut m, 30_000.0, 0.0);
    items.push(m);

    let mut n = Item::new(Kind::Activity);
    n.title = "Day trip to Nikko".into();
    n.start = Some(When::day(date(2027, 5, 17)));
    n.notes = "Tobu line from Asakusa, about 2 hours.".into();
    yen(&mut n, 12_000.0, 0.0);
    items.push(n);

    let mut s = Item::new(Kind::Travel);
    s.status = Status::Booked;
    s.start = at(date(2027, 5, 19), time(10, 0, 0, 0), TYO);
    s.end = at(date(2027, 5, 19), time(12, 15, 0, 0), TYO);
    s.travel = Some(Travel {
        mode: Mode::Train,
        carrier: "Shinkansen Nozomi".into(),
        number: "215".into(),
        from: "Tokyo".into(),
        to: "Kyoto".into(),
        terminal: String::new(),
        seat: "Car 7, 12D/E".into(),
    });
    s.reference = "2207 1188".into();
    s.notes = "Collect the tickets from a machine at Tokyo station with the reference.".into();
    yen(&mut s, 29_000.0, 29_000.0);
    items.push(s);

    let mut r = Item::new(Kind::Stay);
    r.title = "Ryokan Hanami, Kyoto".into();
    r.status = Status::Booked;
    r.start = at(date(2027, 5, 19), time(15, 0, 0, 0), TYO);
    r.end = at(date(2027, 5, 22), time(10, 0, 0, 0), TYO);
    r.address = "Higashiyama, Kyoto".into();
    r.email = "stay@example.com".into();
    r.reference = "HN-20519".into();
    r.notes = "Dinner included on the first night, 18:30.".into();
    yen(&mut r, 168_000.0, 50_000.0);
    r.cost.due = Some(date(2027, 5, 5));
    items.push(r);

    let mut w = Item::new(Kind::Activity);
    w.title = "Fushimi Inari at dawn".into();
    w.start = at(date(2027, 5, 20), time(6, 30, 0, 0), TYO);
    items.push(w);

    let mut o = Item::new(Kind::Travel);
    o.status = Status::Idea;
    o.start = at(date(2027, 5, 22), time(11, 0, 0, 0), TYO);
    o.end = at(date(2027, 5, 22), time(11, 30, 0, 0), TYO);
    o.travel = Some(Travel {
        mode: Mode::Train,
        carrier: "JR Special Rapid".into(),
        from: "Kyoto".into(),
        to: "Osaka".into(),
        ..Default::default()
    });
    yen(&mut o, 1_160.0, 0.0);
    items.push(o);

    let mut k = Item::new(Kind::Stay);
    k.title = "Osaka Bay Hotel".into();
    k.status = Status::Confirmed;
    k.start = Some(When::day(date(2027, 5, 22)));
    k.end = Some(When::day(date(2027, 5, 23)));
    k.reference = "OBH-7731".into();
    yen(&mut k, 21_000.0, 21_000.0);
    k.cost.locked_rate = Some(0.0101);
    items.push(k);

    let mut b = Item::new(Kind::Travel);
    b.status = Status::Paid;
    b.start = at(date(2027, 5, 24), time(21, 30, 0, 0), TYO);
    b.end = at(date(2027, 5, 25), time(8, 55, 0, 0), SYD);
    b.travel = Some(Travel {
        mode: Mode::Flight,
        carrier: "QF".into(),
        number: "34".into(),
        from: "Osaka Kansai (KIX)".into(),
        to: "Sydney (SYD)".into(),
        ..Default::default()
    });
    b.reference = "QX7K2P".into();
    items.push(b);

    let mut e = Item::new(Kind::Other);
    e.title = "eSIM for two".into();
    e.cost.amount = 45.0;
    items.push(e);

    t.items = items;
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_has_one_missing_night() {
        let t = trip();
        let w = crate::checks::check(&t, date(2027, 3, 1));
        let missing: Vec<_> = w.iter().filter(|w| w.text.starts_with("Nowhere")).collect();
        assert_eq!(missing.len(), 1, "{w:#?}");
        assert!(missing[0].text.contains("23 May"));
    }
}
