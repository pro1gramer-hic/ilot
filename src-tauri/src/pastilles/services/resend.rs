//! Resend: e-mails sent today and bounces, from the e-mail list and each one's
//! `last_event` (https://resend.com/docs/api-reference/emails/list-emails,
//! https://resend.com/docs/dashboard/emails/manage-emails). Resend refuses a request
//! without a User-Agent (https://resend.com/docs/api-reference/introduction). Its keys are
//! either « full access » or « sending access »: none is read-only
//! (https://resend.com/docs/api-reference/api-keys/create-api-key). Limit: 10 requests a
//! second for the whole team, shared with the user's own app: one read every 5 minutes.

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, count_label, parse_time, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, USER_AGENT, join};
use std::time::Duration;

pub struct Resend;

static SPEC: Spec = Spec {
    id: "resend",
    name: "Resend",
    palette: "lavande",
    kind: "ecran",
    address: Address::Fixed("https://api.resend.com"),
    account: None,
    refresh: Duration::from_secs(300),
    token_label: Text::ResendTokenLabel,
    rights: Text::ResendRights,
    warning: Some(Text::ResendWarning),
    help: "https://resend.com/api-keys",
};

const PAGE: usize = 100;

impl Service for Resend {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        let limit = PAGE.to_string();
        vec![Request::get(join(ctx.base, "/emails", &[("limit", limit.as_str())])).bearer(ctx.token).header("User-Agent", USER_AGENT)]
    }

    fn summarize(&self, ctx: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [reply] = answers else { return Err(Failure::BadAnswer) };
        let json = reply.json()?;
        let list = json["data"].as_array().ok_or(Failure::BadAnswer)?;
        let more = json["has_more"].as_bool().unwrap_or(false);
        let sent_at = |e: &serde_json::Value| parse_time(text(e, "created_at"));
        let today: Vec<&serde_json::Value> = list.iter().filter(|e| sent_at(e).is_some_and(|at| at >= ctx.midnight_ms())).collect();
        let bounced: Vec<&&serde_json::Value> = today.iter().filter(|e| text(e, "last_event") == "bounced").collect();
        let complained = today.iter().filter(|e| text(e, "last_event") == "complained").count();
        let failed = today.iter().filter(|e| text(e, "last_event") == "failed").count();
        let tone = if !bounced.is_empty() || complained > 0 || failed > 0 {
            Tone::Warn
        } else if today.is_empty() {
            Tone::Idle
        } else {
            Tone::Ok
        };
        let mut line = if today.is_empty() {
            tr(Text::ResendNoneToday)
        } else {
            tr(Text::ResendToday { n: today.len() as u64 })
        };
        let bounces = || tr(Text::ResendBounces { n: bounced.len() as u64 });
        if !bounced.is_empty() {
            line.push_str(&format!(", {}", bounces()));
        }
        // Subjects only: the addresses stay out of the island.
        let items = bounced
            .iter()
            .take(3)
            .map(|e| Item::new(text(e, "subject"), tr(Text::ResendBounce), sent_at(e)).toned(Tone::Warn))
            .collect();
        Ok(Summary {
            tone,
            line,
            short: if bounced.is_empty() { tr(Text::ResendShortSent { n: today.len() }) } else { bounces() },
            metrics: vec![
                Metric::new(&tr(Text::ResendSent), count_label(today.len(), more && today.len() == list.len())),
                Metric::toned(&tr(Text::ResendBouncesMetric), bounced.len(), if bounced.is_empty() { Tone::Ok } else { Tone::Warn }),
                Metric::toned(&tr(Text::ResendComplaints), complained, if complained > 0 { Tone::Warn } else { Tone::Idle }),
            ],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    /// The documented list fields (id, to, from, created_at, subject, last_event).
    const EMAILS: &str = r#"{"object":"list","has_more":false,"data":[
        {"id":"4ef9a417-02e9-4d39-ad75-9611e0fcc33c","to":["delivered@resend.dev"],"from":"Acme <onboarding@resend.dev>",
         "created_at":"2026-10-11 10:13:42.674981+00","subject":"Bienvenue","bcc":null,"cc":null,"reply_to":null,
         "last_event":"delivered","scheduled_at":null},
        {"id":"2","to":["x@example.com"],"created_at":"2026-10-11 09:00:00.000000+00","subject":"Ta facture","last_event":"bounced"},
        {"id":"3","created_at":"2026-10-09 09:00:00.000000+00","subject":"Ancien","last_event":"delivered"}]}"#;

    #[test]
    fn sent_today_and_bounces() {
        let asked = Resend.plan(&ctx("https://api.resend.com", ""), &[]);
        assert_eq!(asked[0].url, "https://api.resend.com/emails?limit=100");
        assert!(asked[0].headers.iter().any(|(name, _)| *name == "User-Agent"));
        let script = Script::new(vec![ok(EMAILS)]);
        let summary = run(&Resend, &script, &ctx("https://api.resend.com", "")).unwrap();
        assert_eq!(summary.tone, Tone::Warn);
        assert_eq!(summary.line, "2 e-mails envoyés aujourd'hui, 1 rebond");
        assert_eq!(summary.items[0].title, "Ta facture");
        // No address leaves the core.
        assert!(!serde_json::to_string(&summary).unwrap().contains("@"));
    }
}
