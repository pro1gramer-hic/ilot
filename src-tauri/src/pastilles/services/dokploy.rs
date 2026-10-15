//! Dokploy, on the user's own instance: applications and compose services with their
//! state, from `GET /api/project.all` and the `x-api-key` header
//! (https://docs.dokploy.com/docs/api, https://docs.dokploy.com/docs/api/reference-project).
//! The documentation shows every answer as `{}`: the shape below is not documented, so
//! the reading is defensive. It walks the answer and keeps any object that carries an
//! `applicationStatus` or `composeStatus` (`idle`, `running`, `done`, `error`, values seen
//! in https://docs.dokploy.com/docs/api/reference-application); anything else is ignored.

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use serde_json::Value;
use std::time::Duration;

pub struct Dokploy;

static SPEC: Spec = Spec {
    id: "dokploy",
    name: "Dokploy",
    palette: "ciel",
    kind: "ecran",
    address: Address::Custom { default: "http://localhost:3000", required: true },
    account: None,
    refresh: Duration::from_secs(120),
    token_label: Text::DokployTokenLabel,
    rights: Text::DokployRights,
    warning: Some(Text::DokployWarning),
    help: "https://docs.dokploy.com/docs/api",
};

/// The walk stops this deep or after this many values: an odd answer stays cheap.
const MAX_DEPTH: usize = 8;
const MAX_NODES: usize = 5_000;

#[derive(Debug, PartialEq)]
struct App {
    name: String,
    status: String,
}

fn walk(value: &Value, depth: usize, seen: &mut usize, apps: &mut Vec<App>) {
    *seen += 1;
    if depth > MAX_DEPTH || *seen > MAX_NODES {
        return;
    }
    match value {
        Value::Object(map) => {
            let status = ["applicationStatus", "composeStatus"]
                .iter()
                .find_map(|key| map.get(*key).and_then(Value::as_str));
            if let Some(status) = status {
                let name = ["name", "appName"].iter().find_map(|key| map.get(*key).and_then(Value::as_str));
                apps.push(App { name: name.map_or_else(|| tr(Text::DokployUnnamed), str::to_owned), status: status.to_owned() });
            }
            for child in map.values() {
                walk(child, depth + 1, seen, apps);
            }
        }
        Value::Array(list) => {
            for child in list {
                walk(child, depth + 1, seen, apps);
            }
        }
        _ => {}
    }
}

fn label_of(status: &str) -> String {
    tr(match status {
        "done" => Text::StateLive,
        "running" => Text::StateDeploying,
        "error" => Text::StateFailed,
        "idle" => Text::StateIdle,
        _ => Text::StateUnknown,
    })
}

fn tone_of(status: &str) -> Tone {
    match status {
        "done" => Tone::Ok,
        "running" => Tone::Busy,
        "error" => Tone::Err,
        _ => Tone::Idle,
    }
}

impl Service for Dokploy {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        vec![Request::get(join(ctx.base, "/api/project.all", &[])).header("x-api-key", ctx.token)]
    }

    fn summarize(&self, _: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [reply] = answers else { return Err(Failure::BadAnswer) };
        let json = reply.json()?;
        let mut apps = Vec::new();
        walk(&json, 0, &mut 0, &mut apps);
        let count = |status: &str| apps.iter().filter(|a| a.status == status).count();
        let (failed, deploying, online) = (count("error"), count("running"), count("done"));
        let tone = if failed > 0 {
            Tone::Err
        } else if deploying > 0 {
            Tone::Busy
        } else if online > 0 {
            Tone::Ok
        } else {
            Tone::Idle
        };
        let line = if apps.is_empty() {
            tr(Text::DokployNone)
        } else {
            let mut parts = vec![tr(Text::DokployApps { n: apps.len() as u64 })];
            if failed > 0 {
                parts.push(tr(Text::CountFailed { n: failed }));
            }
            if deploying > 0 {
                parts.push(tr(Text::CountDeploying { n: deploying }));
            }
            parts.join(", ")
        };
        // The most urgent first: failures, then deployments, then the rest.
        let rank = |a: &App| match a.status.as_str() {
            "error" => 0,
            "running" => 1,
            _ => 2,
        };
        let mut sorted: Vec<&App> = apps.iter().collect();
        sorted.sort_by_key(|a| rank(a));
        let items = sorted.iter().take(3).map(|a| Item::new(a.name.clone(), label_of(&a.status), None).toned(tone_of(&a.status))).collect();
        Ok(Summary {
            tone,
            line,
            short: if failed > 0 {
                tr(Text::CountFailed { n: failed })
            } else if deploying > 0 {
                tr(Text::Deploying)
            } else {
                tr(Text::CountLive { n: online })
            },
            metrics: vec![
                Metric::toned(&tr(Text::MetricLive), online, Tone::Ok),
                Metric::toned(&tr(Text::MetricRunning), deploying, if deploying > 0 { Tone::Busy } else { Tone::Idle }),
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

    /// Not a documented example (there is none): the shape a Dokploy project list is
    /// expected to have, with applications and compose services nested in environments.
    const PROJECTS: &str = r#"[{"projectId":"p1","name":"Boutique","environments":[{"name":"production",
        "applications":[{"applicationId":"a1","name":"api","appName":"boutique-api","applicationStatus":"done"},
                        {"applicationId":"a2","name":"front","applicationStatus":"error"}],
        "compose":[{"composeId":"c1","name":"base","composeStatus":"running"}]}]}]"#;

    #[test]
    fn applications_are_found_wherever_they_sit() {
        let asked = Dokploy.plan(&ctx("https://dokploy.example.com", ""), &[]);
        assert_eq!(asked[0].url, "https://dokploy.example.com/api/project.all");
        assert_eq!(asked[0].headers, [("x-api-key", "jeton-de-test".to_owned())]);
        let script = Script::new(vec![ok(PROJECTS)]);
        let summary = run(&Dokploy, &script, &ctx("https://dokploy.example.com", "")).unwrap();
        assert_eq!(summary.tone, Tone::Err);
        assert_eq!(summary.line, "3 applications, 1 en échec, 1 en déploiement");
        assert_eq!(summary.items[0].title, "front");
        assert_eq!(summary.items[1].detail, "en déploiement");
    }

    #[test]
    fn an_unknown_shape_is_calm() {
        let script = Script::new(vec![ok(r#"{"projects":{"weird":true}}"#)]);
        let summary = run(&Dokploy, &script, &ctx("https://dokploy.example.com", "")).unwrap();
        assert_eq!((summary.tone, summary.line.as_str()), (Tone::Idle, "Aucune application trouvée"));
    }

    #[test]
    fn a_deep_answer_stays_cheap() {
        let mut deep = String::new();
        for _ in 0..100 {
            deep.push_str("{\"a\":");
        }
        deep.push_str("{\"applicationStatus\":\"error\"}");
        deep.push_str(&"}".repeat(100));
        let script = Script::new(vec![ok(&deep)]);
        let summary = run(&Dokploy, &script, &ctx("https://dokploy.example.com", "")).unwrap();
        assert_eq!(summary.line, "Aucune application trouvée");
    }
}
