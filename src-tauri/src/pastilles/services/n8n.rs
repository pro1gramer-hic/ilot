//! n8n, on the user's instance (or n8n cloud): the latest workflow executions and their
//! failures, `GET /api/v1/executions` with the `X-N8N-API-KEY` header
//! (https://docs.n8n.io/api/authentication/, https://docs.n8n.io/connect/n8n-api/executions.md).
//! Execution data stays out (`includeData` is false by default). Scopes such as
//! `execution:read` exist on Enterprise only: other keys have every right.

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, count_label, parse_time, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use std::time::Duration;

pub struct N8n;

static SPEC: Spec = Spec {
    id: "n8n",
    name: "n8n",
    palette: "peche",
    kind: "ecran",
    address: Address::Custom { default: "http://localhost:5678", required: true },
    account: None,
    refresh: Duration::from_secs(120),
    token_label: Text::N8nTokenLabel,
    rights: Text::N8nRights,
    warning: Some(Text::N8nWarning),
    help: "https://docs.n8n.io/api/authentication/",
};

const PAGE: usize = 50;
const FAILED: [&str; 2] = ["error", "crashed"];
const RUNNING: [&str; 3] = ["running", "waiting", "new"];

impl Service for N8n {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        let limit = PAGE.to_string();
        vec![Request::get(join(ctx.base, "/api/v1/executions", &[("limit", limit.as_str())])).header("X-N8N-API-KEY", ctx.token)]
    }

    fn summarize(&self, ctx: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [reply] = answers else { return Err(Failure::BadAnswer) };
        let json = reply.json()?;
        let list = json["data"].as_array().ok_or(Failure::BadAnswer)?;
        let started = |e: &serde_json::Value| parse_time(text(e, "startedAt"));
        let today: Vec<&serde_json::Value> = list.iter().filter(|e| started(e).is_some_and(|at| at >= ctx.midnight_ms())).collect();
        let failed: Vec<&serde_json::Value> = list.iter().filter(|e| FAILED.contains(&text(e, "status"))).collect();
        let failed_today = today.iter().filter(|e| FAILED.contains(&text(e, "status"))).count();
        let running = list.iter().filter(|e| RUNNING.contains(&text(e, "status"))).count();
        let last_failed = list.first().is_some_and(|e| FAILED.contains(&text(e, "status")));
        let tone = if last_failed {
            Tone::Err
        } else if failed_today > 0 {
            Tone::Warn
        } else if running > 0 {
            Tone::Busy
        } else if list.is_empty() {
            Tone::Idle
        } else {
            Tone::Ok
        };
        let mut line = if today.is_empty() {
            tr(Text::N8nNoneToday)
        } else {
            tr(Text::N8nToday { n: today.len() as u64 })
        };
        if failed_today > 0 {
            line.push_str(&format!(", {}", tr(Text::CountFailed { n: failed_today })));
        }
        // The execution names its workflow by id only (the name needs another endpoint).
        let items = failed
            .iter()
            .take(3)
            .map(|e| Item::new(tr(Text::N8nWorkflow { id: text(e, "workflowId") }), tr(Text::N8nFailedExecution), started(e)).toned(Tone::Err))
            .collect();
        Ok(Summary {
            tone,
            line,
            short: tr(if failed_today > 0 { Text::CountFailed { n: failed_today } } else { Text::N8nShort { n: today.len() } }),
            metrics: vec![
                Metric::new(&tr(Text::MetricToday), count_label(today.len(), today.len() == list.len() && list.len() >= PAGE)),
                Metric::toned(&tr(Text::MetricFailed), failed_today, if failed_today > 0 { Tone::Err } else { Tone::Ok }),
                Metric::toned(&tr(Text::MetricRunning), running, if running > 0 { Tone::Busy } else { Tone::Idle }),
            ],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    /// Documented execution fields (id, status, mode, startedAt, stoppedAt, workflowId).
    const EXECUTIONS: &str = r#"{"data":[
        {"id":"1002","status":"success","mode":"trigger","startedAt":"2026-10-11T11:50:00.000Z","stoppedAt":"2026-10-11T11:50:02.000Z","workflowId":"7","retryOf":null},
        {"id":"1001","status":"error","mode":"trigger","startedAt":"2026-10-11T09:00:00.000Z","stoppedAt":"2026-10-11T09:00:01.000Z","workflowId":"3"},
        {"id":"900","status":"error","startedAt":"2026-10-09T09:00:00.000Z","workflowId":"3"}],
        "nextCursor":"MTIzZTQ1NjctZTg5Yi0xMmQzLWE0NTYtNDI2NjE0MTc0MDA"}"#;

    #[test]
    fn executions_and_failures_of_the_day() {
        let asked = N8n.plan(&ctx("https://n8n.example.com", ""), &[]);
        assert_eq!(asked[0].url, "https://n8n.example.com/api/v1/executions?limit=50");
        assert_eq!(asked[0].headers, [("X-N8N-API-KEY", "jeton-de-test".to_owned())]);
        let script = Script::new(vec![ok(EXECUTIONS)]);
        let summary = run(&N8n, &script, &ctx("https://n8n.example.com", "")).unwrap();
        assert_eq!(summary.tone, Tone::Warn);
        assert_eq!(summary.line, "2 exécutions aujourd'hui, 1 en échec");
        assert_eq!(summary.items.len(), 2);
        assert_eq!(summary.items[0].title, "Workflow 3");
    }
}
