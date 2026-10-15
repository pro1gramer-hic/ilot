//! Stripe: today's payments and their amount, from the PaymentIntents created since local
//! midnight (https://docs.stripe.com/api/payment_intents/list). A restricted key with
//! « PaymentIntents: Read » only (https://docs.stripe.com/keys/restricted-api-keys).
//! Read quota: 500 reads per transaction over 30 days, 10 000 a month at least
//! (https://docs.stripe.com/rate-limits): one read every ten minutes is about 4 300 a month.

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use std::collections::BTreeMap;
use std::time::Duration;

pub struct Stripe;

static SPEC: Spec = Spec {
    id: "stripe",
    name: "Stripe",
    palette: "lavande",
    kind: "oreilles",
    address: Address::Fixed("https://api.stripe.com"),
    account: None,
    refresh: Duration::from_secs(600),
    token_label: Text::StripeTokenLabel,
    rights: Text::StripeRights,
    warning: None,
    help: "https://dashboard.stripe.com/apikeys",
};

/// Currencies without minor unit (https://docs.stripe.com/currencies#zero-decimal).
const ZERO_DECIMAL: [&str; 16] =
    ["bif", "clp", "djf", "gnf", "jpy", "kmf", "krw", "mga", "pyg", "rwf", "ugx", "vnd", "vuv", "xaf", "xof", "xpf"];

/// « 1 234,50 € », French style: narrow spaces between thousands, a decimal comma.
pub fn money(minor: i64, currency: &str) -> String {
    let currency = currency.to_lowercase();
    let decimals = if ZERO_DECIMAL.contains(&currency.as_str()) { 0 } else { 2 };
    let unit = 10_i64.pow(decimals);
    let (whole, cents) = (minor.abs() / unit, minor.abs() % unit);
    let sign = if minor < 0 { "-" } else { "" };
    let cents = if decimals == 0 { String::new() } else { format!("{cents:02}") };
    let symbol = match currency.as_str() {
        "eur" => "€".to_owned(),
        "usd" => "$".to_owned(),
        "gbp" => "£".to_owned(),
        other => other.to_uppercase(),
    };
    tr(Text::Money { sign, whole: whole as u64, cents: &cents, symbol: &symbol })
}

impl Service for Stripe {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        let since = (ctx.midnight_ms() / 1000).to_string();
        let query = [("created[gte]", since.as_str()), ("limit", "100")];
        vec![Request::get(join(ctx.base, "/v1/payment_intents", &query)).bearer(ctx.token)]
    }

    fn summarize(&self, ctx: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [reply] = answers else { return Err(Failure::BadAnswer) };
        let json = reply.json()?;
        let list = json["data"].as_array().ok_or(Failure::BadAnswer)?;
        let more = json["has_more"].as_bool().unwrap_or(false);
        let succeeded: Vec<&serde_json::Value> = list.iter().filter(|p| text(p, "status") == "succeeded").collect();
        let failed = list
            .iter()
            .filter(|p| text(p, "status") == "requires_payment_method" && !p["last_payment_error"].is_null())
            .count();
        let mut totals: BTreeMap<String, (i64, usize)> = BTreeMap::new();
        for p in &succeeded {
            let entry = totals.entry(text(p, "currency").to_owned()).or_default();
            entry.0 += p["amount_received"].as_i64().unwrap_or(0);
            entry.1 += 1;
        }
        // The currency paid most often today leads; others are mentioned.
        let main = totals.iter().max_by_key(|(_, (_, n))| *n).map(|(c, (amount, _))| money(*amount, c));
        let amount = main.clone().unwrap_or_else(|| money(0, "eur"));
        let count = if more { format!("{}+", succeeded.len()) } else { succeeded.len().to_string() };
        let mut line = if succeeded.is_empty() {
            tr(Text::StripeNoneToday)
        } else {
            tr(Text::StripeToday { n: succeeded.len() as u64, amount: &amount })
        };
        if totals.len() > 1 {
            line.push(' ');
            line.push_str(&tr(Text::StripeOtherCurrencies));
        }
        let items = succeeded
            .iter()
            .take(3)
            .map(|p| {
                let at = p["created"].as_i64().map(|s| s * 1000);
                let label = at.map_or_else(|| tr(Text::StripePayment), |ms| tr(Text::StripePaymentAt { clock: &ctx.clock(ms) }));
                let description = text(p, "description");
                Item::new(money(p["amount_received"].as_i64().unwrap_or(0), text(p, "currency")), if description.is_empty() { label } else { description.to_owned() }, at)
                    .toned(Tone::Ok)
            })
            .collect();
        Ok(Summary {
            tone: if failed > 0 {
                Tone::Warn
            } else if succeeded.is_empty() {
                Tone::Idle
            } else {
                Tone::Ok
            },
            line,
            short: amount.clone(),
            metrics: vec![
                Metric::new(&tr(Text::StripePayments), count),
                Metric::new(&tr(Text::StripeAmount), amount),
                Metric::toned(&tr(Text::StripeDeclined), failed, if failed > 0 { Tone::Warn } else { Tone::Idle }),
            ],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    /// Documented PaymentIntent fields (id, amount, amount_received, currency, status,
    /// created, last_payment_error, description).
    const TODAY: &str = r#"{"object":"list","url":"/v1/payment_intents","has_more":false,"data":[
        {"id":"pi_1","object":"payment_intent","amount":4200,"amount_received":4200,"currency":"eur","status":"succeeded",
         "created":1791716400,"last_payment_error":null,"description":null},
        {"id":"pi_2","amount":12050,"amount_received":12050,"currency":"eur","status":"succeeded","created":1791712800},
        {"id":"pi_3","amount":900,"amount_received":0,"currency":"eur","status":"requires_payment_method",
         "last_payment_error":{"code":"card_declined"}}]}"#;

    #[test]
    fn todays_payments_since_local_midnight() {
        let asked = Stripe.plan(&ctx("https://api.stripe.com", ""), &[]);
        // Midnight in Paris = 2026-10-10 22:00 UTC = 1791669600.
        assert_eq!(asked[0].url, "https://api.stripe.com/v1/payment_intents?created%5Bgte%5D=1791669600&limit=100");
        let script = Script::new(vec![ok(TODAY)]);
        let summary = run(&Stripe, &script, &ctx("https://api.stripe.com", "")).unwrap();
        assert_eq!(summary.line, "2 paiements aujourd'hui, 162,50\u{a0}€");
        assert_eq!(summary.tone, Tone::Warn);
        assert_eq!(summary.metrics[2].value, "1");
        assert_eq!(summary.items[0].title, "42,00\u{a0}€");
        assert_eq!(summary.items[0].detail, "Paiement de 13:00");
    }

    #[test]
    fn amounts_read_the_french_way() {
        assert_eq!(money(123_456_789, "eur"), "1\u{202f}234\u{202f}567,89\u{a0}€");
        assert_eq!(money(5, "usd"), "0,05\u{a0}$");
        assert_eq!(money(1500, "jpy"), "1\u{202f}500\u{a0}JPY");
        assert_eq!(money(-250, "gbp"), "-2,50\u{a0}£");
    }

    #[test]
    fn a_quiet_day() {
        let script = Script::new(vec![ok(r#"{"object":"list","has_more":false,"data":[]}"#)]);
        let summary = run(&Stripe, &script, &ctx("https://api.stripe.com", "")).unwrap();
        assert_eq!((summary.tone, summary.line.as_str()), (Tone::Idle, "Aucun paiement aujourd'hui"));
    }

    #[test]
    fn amounts_read_the_english_way() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(money(123_456_789, "eur"), "€1,234,567.89");
        assert_eq!(money(1500, "jpy"), "1,500 JPY");
        assert_eq!(money(-250, "gbp"), "-£2.50");
        let script = Script::new(vec![ok(TODAY)]);
        let summary = run(&Stripe, &script, &ctx("https://api.stripe.com", "")).unwrap();
        assert_eq!(summary.line, "2 payments today, €162.50");
        assert_eq!(summary.items[0].detail, "Payment at 13:00");
    }
}
