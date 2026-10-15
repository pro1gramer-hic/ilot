//! Cal.com: the next bookings, `GET /v2/bookings?status=upcoming&sortStart=asc` with the
//! `cal-api-version` header this endpoint requires
//! (https://cal.com/docs/api-reference/v2/bookings/get-all-bookings). API keys carry many
//! rights; none is read-only (https://cal.com/docs/api-reference/v2/introduction).
//! Limit: 120 requests a minute per key.

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, parse_time, rfc3339, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use std::time::Duration;

pub struct CalCom;

static SPEC: Spec = Spec {
    id: "calcom",
    name: "Cal.com",
    palette: "menthe",
    kind: "meche",
    address: Address::Custom { default: "https://api.cal.com", required: false },
    account: None,
    refresh: Duration::from_secs(600),
    token_label: Text::CalcomTokenLabel,
    rights: Text::CalcomRights,
    warning: Some(Text::CalcomWarning),
    help: "https://app.cal.com/settings/developer/api-keys",
};

const VERSION: &str = "2026-05-01";
const DAY_MS: i64 = 86_400_000;

/// « aujourd'hui à 15:30 », « demain à 9:00 », « le 14/10 à 10:00 ».
fn when(ctx: &Ctx, at: i64) -> String {
    let day = |ms: i64| (ms + ctx.offset_min as i64 * 60_000).div_euclid(DAY_MS);
    let clock = ctx.clock(at);
    match day(at) - day(ctx.now_ms) {
        0 => tr(Text::CalcomToday { clock: &clock }),
        1 => tr(Text::CalcomTomorrow { clock: &clock }),
        _ => {
            let date = rfc3339(at + ctx.offset_min as i64 * 60_000);
            tr(Text::CalcomOn { day: &date[8..10], month: &date[5..7], clock: &clock })
        }
    }
}

impl Service for CalCom {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        let query = [("status", "upcoming"), ("limit", "5"), ("sortStart", "asc")];
        vec![Request::get(join(ctx.base, "/v2/bookings", &query)).bearer(ctx.token).header("cal-api-version", VERSION)]
    }

    fn summarize(&self, ctx: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [reply] = answers else { return Err(Failure::BadAnswer) };
        let json = reply.json()?;
        let list = json["data"].as_array().ok_or(Failure::BadAnswer)?;
        let start = |b: &serde_json::Value| parse_time(text(b, "start"));
        let tomorrow = ctx.midnight_ms() + DAY_MS;
        let today = list.iter().filter(|b| start(b).is_some_and(|at| at < tomorrow)).count();
        let next = list.first();
        // Titles and times only: attendees' names stay out of the island.
        let items = list
            .iter()
            .take(3)
            .map(|b| {
                let detail = if text(b, "status") == "pending" { Text::CalcomToConfirm } else { Text::CalcomBooking };
                Item::new(text(b, "title"), tr(detail), start(b))
            })
            .collect();
        Ok(Summary {
            tone: if today > 0 { Tone::Busy } else if next.is_some() { Tone::Ok } else { Tone::Idle },
            line: match next.and_then(|b| start(b).map(|at| (b, at))) {
                None => tr(Text::CalcomNone),
                Some((b, at)) => tr(Text::CalcomNext { title: text(b, "title"), when: &when(ctx, at) }),
            },
            short: next.and_then(start).map_or_else(|| tr(Text::CalcomFree), |at| tr(Text::CalcomShort { clock: &ctx.clock(at) })),
            metrics: vec![Metric::new(&tr(Text::MetricToday), today), Metric::new(&tr(Text::MetricUpcoming), list.len())],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    /// The documented example (status, data[].title, start, end, status, attendees, pagination).
    const BOOKINGS: &str = r#"{"status":"success","data":[
        {"id":123,"uid":"booking_uid_123","title":"Consultation","status":"accepted",
         "start":"2026-10-11T13:30:00Z","end":"2026-10-11T14:30:00Z","location":"https://example.com/meeting",
         "attendees":[{"name":"John Doe"}]},
        {"id":124,"title":"Point d'équipe","status":"pending","start":"2026-10-14T08:00:00Z"}],
        "pagination":{"nextCursor":null,"hasMore":false}}"#;

    #[test]
    fn next_bookings_in_local_time() {
        let asked = CalCom.plan(&ctx("https://api.cal.com", ""), &[]);
        assert_eq!(asked[0].url, "https://api.cal.com/v2/bookings?status=upcoming&limit=5&sortStart=asc");
        assert!(asked[0].headers.contains(&("cal-api-version", VERSION.to_owned())));
        let script = Script::new(vec![ok(BOOKINGS)]);
        let summary = run(&CalCom, &script, &ctx("https://api.cal.com", "")).unwrap();
        assert_eq!(summary.line, "Prochain : Consultation, aujourd'hui à 15:30");
        assert_eq!(summary.short, "RDV 15:30");
        assert_eq!(summary.items[1].detail, "À confirmer");
        assert!(!serde_json::to_string(&summary).unwrap().contains("John"));
    }

    #[test]
    fn days_are_named_simply() {
        let c = ctx("", "");
        assert_eq!(when(&c, NOW + DAY_MS), "demain à 14:00");
        assert_eq!(when(&c, NOW + 3 * DAY_MS), "le 14/10 à 14:00");
    }

    #[test]
    fn days_are_named_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let c = ctx("", "");
        assert_eq!(when(&c, NOW + DAY_MS), "tomorrow at 14:00");
        assert_eq!(when(&c, NOW + 3 * DAY_MS), "on Oct 14 at 14:00");
        let summary = run(&CalCom, &Script::new(vec![ok(BOOKINGS)]), &ctx("https://api.cal.com", "")).unwrap();
        assert_eq!(summary.line, "Next: Consultation, today at 15:30");
        assert_eq!(summary.items[1].detail, "To confirm");
    }
}
