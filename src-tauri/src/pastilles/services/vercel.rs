//! Vercel: the latest deployments and their state, `GET /v7/deployments`
//! (https://vercel.com/docs/rest-api/deployments/list-deployments). Tokens are scoped to
//! an account, a team or one project, never read-only (https://vercel.com/docs/accounts/access-tokens):
//! the narrowest scope is asked for. Limit: 1 000 lists a minute (https://vercel.com/docs/limits).

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use std::time::Duration;

pub struct Vercel;

static SPEC: Spec = Spec {
    id: "vercel",
    name: "Vercel",
    palette: "nuit",
    kind: "ecran",
    address: Address::Fixed("https://api.vercel.com"),
    account: None,
    refresh: Duration::from_secs(120),
    token_label: Text::VercelTokenLabel,
    rights: Text::VercelRights,
    warning: Some(Text::VercelWarning),
    help: "https://vercel.com/account/settings/tokens",
};

const BUILDING: [&str; 3] = ["BUILDING", "QUEUED", "INITIALIZING"];

/// The state of a deployment, in French.
fn state_label(state: &str) -> String {
    tr(match state {
        "READY" => Text::StateLive,
        "ERROR" => Text::StateFailed,
        "CANCELED" => Text::StateCanceled,
        "BLOCKED" => Text::StateBlocked,
        "DELETED" => Text::StateDeleted,
        s if BUILDING.contains(&s) => Text::StateBuilding,
        _ => Text::StateUnknown,
    })
}

fn tone_of(state: &str) -> Tone {
    match state {
        "READY" => Tone::Ok,
        "ERROR" | "BLOCKED" => Tone::Err,
        s if BUILDING.contains(&s) => Tone::Busy,
        _ => Tone::Idle,
    }
}

impl Service for Vercel {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        vec![Request::get(join(ctx.base, "/v7/deployments", &[("limit", "10")])).bearer(ctx.token)]
    }

    fn summarize(&self, _: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [reply] = answers else { return Err(Failure::BadAnswer) };
        let json = reply.json()?;
        let list = json["deployments"].as_array().ok_or(Failure::BadAnswer)?;
        // `readyState` in the list (`state` is its other name in the documentation).
        let state = |d: &serde_json::Value| -> String {
            let s = text(d, "readyState");
            if s.is_empty() { text(d, "state").to_owned() } else { s.to_owned() }
        };
        let building = list.iter().filter(|d| BUILDING.contains(&state(d).as_str())).count();
        let failed = list.iter().filter(|d| state(d) == "ERROR").count();
        let latest = list.first();
        let tone = latest.map_or(Tone::Idle, |d| if building > 0 { Tone::Busy } else { tone_of(&state(d)) });
        let line = match latest {
            None => tr(Text::VercelNone),
            Some(d) => tr(Text::VercelLatest { state: &state_label(&state(d)), name: text(d, "name") }),
        };
        let items = list
            .iter()
            .take(3)
            .map(|d| {
                let target = tr(if text(d, "target") == "production" { Text::VercelProduction } else { Text::VercelPreview });
                Item::new(text(d, "name"), format!("{target}, {}", state_label(&state(d))), d["created"].as_i64())
                    .toned(tone_of(&state(d)))
            })
            .collect();
        Ok(Summary {
            tone,
            line,
            short: match (building, latest) {
                (n, _) if n > 0 => tr(Text::Deploying),
                (_, Some(d)) => format!("Vercel {}", state_label(&state(d))),
                _ => "Vercel".to_owned(),
            },
            metrics: vec![
                Metric::new(&tr(Text::VercelLatestMetric), list.len()),
                Metric::toned(&tr(Text::MetricRunning), building, if building > 0 { Tone::Busy } else { Tone::Idle }),
                Metric::toned(&tr(Text::MetricFailed), failed, if failed > 0 { Tone::Err } else { Tone::Ok }),
            ],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    /// The documented example of `GET /v7/deployments` (fields kept: uid, name, url, created,
    /// readyState, target, inspectorUrl, errorCode).
    const LIST: &str = r#"{"deployments":[
        {"uid":"dpl_2euZBFqxYdDMDG1jTrHFnNZ2eUVa","name":"docs","url":"docs-9jaeg38me.vercel.app","created":1609492210000,
         "readyState":"READY","target":"production","inspectorUrl":"https://vercel.com/acme/nextjs/J1hXN00qjUeoYfpEEf7dnDtpSiVq"},
        {"uid":"dpl_2","name":"docs","created":1609492000000,"readyState":"ERROR","target":null,"errorCode":"BUILD_FAILED"}],
        "pagination":{"count":20,"next":1540095775951,"prev":1540095775951}}"#;

    #[test]
    fn latest_deployments() {
        let asked = Vercel.plan(&ctx("https://api.vercel.com", ""), &[]);
        assert_eq!(asked[0].url, "https://api.vercel.com/v7/deployments?limit=10");
        let script = Script::new(vec![ok(LIST)]);
        let summary = run(&Vercel, &script, &ctx("https://api.vercel.com", "")).unwrap();
        assert_eq!(summary.tone, Tone::Ok);
        assert_eq!(summary.line, "Dernier déploiement en ligne : docs");
        assert_eq!(summary.items[1].detail, "Aperçu, en échec");
        assert_eq!(summary.items[1].tone, Some(Tone::Err));
        assert_eq!(summary.metrics[2].value, "1");
    }

    #[test]
    fn a_build_in_progress_shows() {
        let script = Script::new(vec![ok(r#"{"deployments":[{"name":"site","readyState":"BUILDING","created":1}]}"#)]);
        let summary = run(&Vercel, &script, &ctx("https://api.vercel.com", "")).unwrap();
        assert_eq!((summary.tone, summary.short.as_str()), (Tone::Busy, "Déploiement…"));
    }

    #[test]
    fn no_deployment_yet() {
        let script = Script::new(vec![ok(r#"{"deployments":[],"pagination":{}}"#)]);
        let summary = run(&Vercel, &script, &ctx("https://api.vercel.com", "")).unwrap();
        assert_eq!((summary.tone, summary.line.as_str()), (Tone::Idle, "Aucun déploiement"));
    }
}
