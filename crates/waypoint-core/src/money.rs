//! Currencies, amounts in the home currency, and the trip's totals.

use crate::model::{Item, Kind, Status, Trip};

/// A currency: its code, name, symbol and how many decimal places it uses.
pub struct Currency {
    pub code: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
    pub decimals: u8,
}

const fn c(code: &'static str, name: &'static str, symbol: &'static str, decimals: u8) -> Currency {
    Currency {
        code,
        name,
        symbol,
        decimals,
    }
}

/// The currencies offered in lists. Any other three-letter code can still be
/// typed in.
pub const CURRENCIES: &[Currency] = &[
    c("AUD", "Australian dollar", "$", 2),
    c("NZD", "New Zealand dollar", "NZ$", 2),
    c("USD", "US dollar", "US$", 2),
    c("CAD", "Canadian dollar", "C$", 2),
    c("EUR", "Euro", "€", 2),
    c("GBP", "Pound sterling", "£", 2),
    c("CHF", "Swiss franc", "CHF ", 2),
    c("NOK", "Norwegian krone", "NOK ", 2),
    c("SEK", "Swedish krona", "SEK ", 2),
    c("DKK", "Danish krone", "DKK ", 2),
    c("ISK", "Icelandic króna", "ISK ", 0),
    c("CZK", "Czech koruna", "CZK ", 2),
    c("HUF", "Hungarian forint", "HUF ", 0),
    c("PLN", "Polish złoty", "PLN ", 2),
    c("TRY", "Turkish lira", "TRY ", 2),
    c("JPY", "Japanese yen", "¥", 0),
    c("KRW", "South Korean won", "₩", 0),
    c("CNY", "Chinese yuan", "CN¥", 2),
    c("HKD", "Hong Kong dollar", "HK$", 2),
    c("TWD", "New Taiwan dollar", "NT$", 0),
    c("SGD", "Singapore dollar", "S$", 2),
    c("MYR", "Malaysian ringgit", "RM ", 2),
    c("THB", "Thai baht", "฿", 2),
    c("IDR", "Indonesian rupiah", "Rp ", 0),
    c("VND", "Vietnamese đồng", "₫", 0),
    c("PHP", "Philippine peso", "₱", 2),
    c("INR", "Indian rupee", "₹", 2),
    c("LKR", "Sri Lankan rupee", "LKR ", 2),
    c("AED", "UAE dirham", "AED ", 2),
    c("FJD", "Fijian dollar", "FJ$", 2),
    c("XPF", "CFP franc", "XPF ", 0),
    c("ZAR", "South African rand", "R ", 2),
    c("MXN", "Mexican peso", "MX$", 2),
    c("BRL", "Brazilian real", "R$", 2),
];

pub fn currency(code: &str) -> Option<&'static Currency> {
    CURRENCIES
        .iter()
        .find(|c| c.code.eq_ignore_ascii_case(code))
}

/// "¥84,000", "$1,234.50", "XYZ 12.00".
pub fn format(amount: f64, code: &str) -> String {
    let (symbol, decimals) = match currency(code) {
        Some(c) => (c.symbol.to_string(), c.decimals),
        None if code.is_empty() => (String::new(), 2),
        None => (format!("{} ", code.to_uppercase()), 2),
    };
    let sign = if amount < 0.0 { "−" } else { "" };
    format!("{sign}{symbol}{}", group(amount.abs(), decimals))
}

/// A number with thousands separators: "1,234.50".
pub fn group(v: f64, decimals: u8) -> String {
    let s = format!("{:.*}", decimals as usize, v);
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s.as_str(), None),
    };
    let mut out = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    out
}

/// Reads an amount typed by a person: "1,234.50", "$80", " 12 ".
pub fn parse_amount(s: &str) -> Option<f64> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}

/// What turns an item's amount into the home currency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateUsed {
    /// It's already in the home currency.
    Home,
    /// Locked when it was paid.
    Locked(f64),
    /// The trip's current rate, an estimate until it's paid.
    Current(f64),
    /// No rate yet.
    Missing,
}

impl RateUsed {
    pub fn factor(self) -> Option<f64> {
        match self {
            RateUsed::Home => Some(1.0),
            RateUsed::Locked(r) | RateUsed::Current(r) => Some(r),
            RateUsed::Missing => None,
        }
    }
}

impl Trip {
    /// An item's currency, with an empty one read as the home currency.
    pub fn currency_of(&self, item: &Item) -> String {
        let c = item.cost.currency.trim().to_uppercase();
        if c.is_empty() {
            self.home_currency.clone()
        } else {
            c
        }
    }

    pub fn rate_for(&self, item: &Item) -> RateUsed {
        let code = self.currency_of(item);
        if code == self.home_currency {
            RateUsed::Home
        } else if let Some(r) = item.cost.locked_rate {
            RateUsed::Locked(r)
        } else {
            match self.rates.get(&code) {
                Some(r) if r.home_per_unit > 0.0 => RateUsed::Current(r.home_per_unit),
                _ => RateUsed::Missing,
            }
        }
    }

    /// An item's whole cost in the home currency.
    pub fn home_amount(&self, item: &Item) -> Option<f64> {
        self.rate_for(item).factor().map(|r| item.cost.amount * r)
    }

    /// Lock the current rate on an item, as it's paid.
    pub fn lock_rate(&mut self, id: &str) {
        let Some(idx) = self.items.iter().position(|i| i.id == id) else {
            return;
        };
        if let RateUsed::Current(r) = self.rate_for(&self.items[idx]) {
            self.items[idx].cost.locked_rate = Some(r);
        }
    }

    /// Set an item's status. Marking it paid also marks the whole amount paid
    /// and locks the exchange rate.
    pub fn set_status(&mut self, id: &str, status: Status) {
        let Some(item) = self.item_mut(id) else {
            return;
        };
        item.status = status;
        if status == Status::Paid {
            item.cost.paid = item.cost.amount.max(item.cost.paid);
            self.lock_rate(id);
        }
    }

    pub fn totals(&self) -> Totals {
        let mut t = Totals::default();
        for item in &self.items {
            if item.cost.is_empty() {
                continue;
            }
            let Some(rate) = self.rate_for(item).factor() else {
                t.unpriced += 1;
                continue;
            };
            let whole = item.cost.amount * rate;
            let paid = item.cost.paid.min(item.cost.amount).max(0.0) * rate;
            match item.status {
                Status::Idea => t.ideas += whole,
                Status::Cancelled => t.lost += paid,
                _ => {
                    t.committed += whole;
                    t.paid += paid;
                    *t.by_kind.entry(item.kind).or_default() += whole;
                }
            }
        }
        t
    }
}

/// The trip's money, in the home currency.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Totals {
    /// Booked, confirmed and paid items, whole cost.
    pub committed: f64,
    /// Paid so far, on those.
    pub paid: f64,
    /// Ideas not yet booked.
    pub ideas: f64,
    /// Paid on cancelled items: deposits and fees not coming back.
    pub lost: f64,
    /// Items with a cost but no exchange rate, left out of the sums.
    pub unpriced: usize,
    /// Committed cost by kind.
    pub by_kind: std::collections::BTreeMap<Kind, f64>,
}

impl Totals {
    /// Still to pay on what's booked.
    pub fn owing(&self) -> f64 {
        (self.committed - self.paid).max(0.0)
    }

    /// Everything spent or to be spent: committed plus what cancellations
    /// cost.
    pub fn spend(&self) -> f64 {
        self.committed + self.lost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Item, Kind, Rate};

    fn trip() -> Trip {
        let mut t = Trip::new("T");
        t.rates.insert(
            "JPY".into(),
            Rate {
                home_per_unit: 0.01,
                ..Default::default()
            },
        );
        t
    }

    #[test]
    fn formats() {
        assert_eq!(format(84000.0, "JPY"), "¥84,000");
        assert_eq!(format(1234.5, "AUD"), "$1,234.50");
        assert_eq!(format(-12.0, "EUR"), "−€12.00");
        assert_eq!(format(5.0, "xyz"), "XYZ 5.00");
        assert_eq!(group(999.0, 0), "999");
        assert_eq!(group(1000000.0, 0), "1,000,000");
    }

    #[test]
    fn parses_typed_amounts() {
        assert_eq!(parse_amount("1,234.50"), Some(1234.5));
        assert_eq!(parse_amount("$80"), Some(80.0));
        assert_eq!(parse_amount(""), None);
    }

    #[test]
    fn paid_locks_rate_and_totals_split_by_status() {
        let mut t = trip();
        let mut hotel = Item::new(Kind::Stay);
        hotel.status = Status::Booked;
        hotel.cost.currency = "JPY".into();
        hotel.cost.amount = 100_000.0;
        hotel.cost.paid = 20_000.0;
        let id = hotel.id.clone();
        t.items.push(hotel);

        let mut idea = Item::new(Kind::Activity);
        idea.cost.amount = 50.0;
        t.items.push(idea);

        let mut cancelled = Item::new(Kind::Activity);
        cancelled.status = Status::Cancelled;
        cancelled.cost.amount = 200.0;
        cancelled.cost.paid = 40.0;
        t.items.push(cancelled);

        let tot = t.totals();
        assert!((tot.committed - 1000.0).abs() < 1e-9);
        assert!((tot.paid - 200.0).abs() < 1e-9);
        assert!((tot.owing() - 800.0).abs() < 1e-9);
        assert!((tot.ideas - 50.0).abs() < 1e-9);
        assert!((tot.lost - 40.0).abs() < 1e-9);

        t.set_status(&id, Status::Paid);
        // The rate moves, but the paid hotel keeps the one it was paid at.
        t.rates.get_mut("JPY").unwrap().home_per_unit = 0.02;
        let tot = t.totals();
        assert!((tot.committed - 1000.0).abs() < 1e-9);
        assert!((tot.owing()).abs() < 1e-9);
    }

    #[test]
    fn missing_rate_is_counted_not_guessed() {
        let mut t = trip();
        let mut i = Item::new(Kind::Meal);
        i.status = Status::Booked;
        i.cost.currency = "EUR".into();
        i.cost.amount = 80.0;
        t.items.push(i);
        let tot = t.totals();
        assert_eq!(tot.unpriced, 1);
        assert_eq!(tot.committed, 0.0);
    }
}
