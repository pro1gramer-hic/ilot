//! Cloudflare: Pages projects with their latest deployment
//! (https://developers.cloudflare.com/api/resources/pages/subresources/projects/methods/list/),
//! Workers scripts (https://developers.cloudflare.com/api/resources/workers/subresources/scripts/methods/list/)
//! and the alerts sent in the last 24 hours
//! (https://developers.cloudflare.com/api/resources/alerting/subresources/history/methods/list/).
//! Limit: 1 200 requests per 5 minutes for the whole user
//! (https://developers.cloudflare.com/fundamentals/api/reference/limits/): 3 every 5 minutes.

use super::{Account, AccountRule, Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, parse_time, rfc3339, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use std::time::Duration;

pub struct Cloudflare;

static SPEC: Spec = Spec {
    id: "cloudflare",
    name: "Cloudflare",
    palette: "citron",
    kind: "oreilles",
    address: Address::Fixed("https://api.cloudflare.com"),
    account: Some(Account {
        label: Text::CloudflareAccountLabel,
        hint: Text::CloudflareAccountHint,
        rule: AccountRule::Hex32,
    }),
    refresh: Duration::from_secs(300),
    token_label: Text::CloudflareTokenLabel,
    rights: Text::CloudflareRights,
    warning: None,
    help: "https://dash.cloudflare.com/profile/api-tokens",
};

const DAY_MS: i64 = 86_400_000;

fn stage(project: &serde_json::Value) -> (&str, &str) {
    let stage = &project["latest_deployment"]["latest_stage"];
    (text(stage, "name"), text(stage, "status"))
}

fn stage_label(name: &str, status: &str) -> String {
    tr(match status {
        "success" if name == "deploy" => Text::StateLive,
        "failure" => Text::StateFailed,
        "active" | "idle" => Text::StateDeploying,
        "canceled" => Text::StateCanceled,
        _ => Text::StateWaiting,
    })
}

fn stage_tone(name: &str, status: &str) -> Tone {
    match status {
        "failure" => Tone::Err,
        "active" | "idle" => Tone::Busy,
        "success" if name == "deploy" => Tone::Ok,
        _ => Tone::Idle,
    }
}

fn results(reply: &Reply) -> Result<Vec<serde_json::Value>, Failure> {
    let json = reply.json()?;
    if json["success"] == false {
        return Err(Failure::BadAnswer);
    }
    json["result"].as_array().cloned().ok_or(Failure::BadAnswer)
}

impl Service for Cloudflare {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() || ctx.account.is_empty() {
            return Vec::new();
        }
        let account = format!("/client/v4/accounts/{}", ctx.account);
        let since = rfc3339(ctx.now_ms - DAY_MS);
        [
            join(ctx.base, &format!("{account}/pages/projects"), &[("per_page", "10")]),
            join(ctx.base, &format!("{account}/workers/scripts"), &[]),
            join(ctx.base, &format!("{account}/alerting/v3/history"), &[("per_page", "5"), ("since", since.as_str())]),
        ]
        .into_iter()
        .map(|url| Request::get(url).bearer(ctx.token))
        .collect()
    }

    fn summarize(&self, ctx: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        if ctx.account.is_empty() {
            return Err(Failure::Setup(Text::AccountIdMissing));
        }
        let [pages, workers, alerts] = answers else { return Err(Failure::BadAnswer) };
        let (pages, workers, alerts) = (results(pages)?, results(workers)?, results(alerts)?);
        let failing = pages.iter().filter(|p| stage(p).1 == "failure").count();
        let deploying = pages.iter().filter(|p| matches!(stage(p).1, "active" | "idle")).count();
        let tone = if failing > 0 {
            Tone::Err
        } else if !alerts.is_empty() {
            Tone::Warn
        } else if deploying > 0 {
            Tone::Busy
        } else {
            Tone::Ok
        };
        let alert_count = || tr(Text::CloudflareAlerts { n: alerts.len() as u64 });
        let mut parts = vec![tr(Text::CloudflarePages { n: pages.len() as u64 }), tr(Text::CloudflareWorkers { n: workers.len() as u64 })];
        if failing > 0 {
            parts.push(tr(Text::CountFailed { n: failing }));
        }
        if !alerts.is_empty() {
            parts.push(alert_count());
        }
        let mut items: Vec<Item> = alerts
            .iter()
            .take(2)
            .map(|a| Item::new(text(a, "name"), text(a, "alert_type"), parse_time(text(a, "sent"))).toned(Tone::Warn))
            .collect();
        items.extend(pages.iter().take(3 - items.len().min(3)).map(|p| {
            let (name, status) = stage(p);
            let when = parse_time(text(&p["latest_deployment"], "created_on"));
            Item::new(text(p, "name"), format!("Pages, {}", stage_label(name, status)), when).toned(stage_tone(name, status))
        }));
        Ok(Summary {
            tone,
            line: parts.join(", "),
            short: if failing > 0 {
                tr(Text::CountFailed { n: failing })
            } else if !alerts.is_empty() {
                alert_count()
            } else {
                "Cloudflare".to_owned()
            },
            metrics: vec![
                Metric::toned(&tr(Text::CloudflarePagesFailed), failing, if failing > 0 { Tone::Err } else { Tone::Ok }),
                Metric::new(&tr(Text::CloudflareWorkersMetric), workers.len()),
                Metric::toned(&tr(Text::CloudflareAlertsMetric), alerts.len(), if alerts.is_empty() { Tone::Idle } else { Tone::Warn }),
            ],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    const ACCOUNT: &str = "023e105f4ecef8ad9ca31a8372d0c353";

    /// Documented fields of a Pages project (name, latest_deployment.latest_stage).
    const PAGES: &str = r#"{"success":true,"errors":[],"result":[
        {"name":"ninjakittens","latest_deployment":{"id":"f64788e9","created_on":"2026-10-11T11:30:00Z",
         "latest_stage":{"name":"deploy","status":"success"}}},
        {"name":"blog","latest_deployment":{"latest_stage":{"name":"build","status":"failure"}}}],
        "result_info":{"count":2,"page":1,"per_page":20,"total_count":2}}"#;
    const WORKERS: &str = r#"{"success":true,"result":[{"id":"my-workers-script","created_on":"2017-01-01T00:00:00Z","modified_on":"2017-01-01T00:00:00Z"}]}"#;
    const ALERTS: &str = r#"{"success":true,"result":[{"alert_type":"universal_ssl_event_type","name":"SSL Notification Event",
        "description":"Universal Certificate validation status, issuance, renewal, and expiration notices",
        "mechanism_type":"email","sent":"2026-10-11T08:00:00Z"}]}"#;

    #[test]
    fn three_reads_for_the_account() {
        let asked = Cloudflare.plan(&ctx("https://api.cloudflare.com", ACCOUNT), &[]);
        assert_eq!(asked.len(), 3);
        assert_eq!(asked[0].url, format!("https://api.cloudflare.com/client/v4/accounts/{ACCOUNT}/pages/projects?per_page=10"));
        assert!(asked[2].url.contains("since=2026-10-10T12%3A00%3A00Z"));
        // Without the account id, nothing is asked.
        assert!(Cloudflare.plan(&ctx("https://api.cloudflare.com", ""), &[]).is_empty());
    }

    #[test]
    fn pages_workers_and_alerts() {
        let script = Script::new(vec![ok(PAGES), ok(WORKERS), ok(ALERTS)]);
        let summary = run(&Cloudflare, &script, &ctx("https://api.cloudflare.com", ACCOUNT)).unwrap();
        assert_eq!(summary.tone, Tone::Err);
        assert_eq!(summary.line, "2 projets Pages, 1 Worker, 1 en échec, 1 alerte");
        assert_eq!(summary.items[0].title, "SSL Notification Event");
        assert_eq!(summary.items[1].detail, "Pages, en ligne");
        assert_eq!(summary.items[2].tone, Some(Tone::Err));
    }

    #[test]
    fn a_missing_account_is_a_setup_problem() {
        let script = Script::new(vec![]);
        let failure = run(&Cloudflare, &script, &ctx("https://api.cloudflare.com", "")).unwrap_err();
        assert_eq!(failure.line(), "Identifiant du compte manquant");
        assert!(script.asked().is_empty());
    }
}
