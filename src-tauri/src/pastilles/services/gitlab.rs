//! GitLab (gitlab.com or the user's instance): open merge requests, reviews asked of the
//! user (https://docs.gitlab.com/api/merge_requests/, `scope=created_by_me` and
//! `scope=reviews_for_me`), and the user's recent pipelines (https://docs.gitlab.com/api/pipelines/,
//! `GET /pipelines`, GitLab 19.3 and later: an older instance answers 404 and the pastille
//! simply shows no pipeline). Token scope `read_api`
//! (https://docs.gitlab.com/security/tokens/access_token_scopes/).

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, count_label, parse_time, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use std::time::Duration;

pub struct GitLab;

static SPEC: Spec = Spec {
    id: "gitlab",
    name: "GitLab",
    palette: "peche",
    kind: "oreilles",
    address: Address::Custom { default: "https://gitlab.com", required: false },
    account: None,
    refresh: Duration::from_secs(180),
    token_label: Text::GitlabTokenLabel,
    rights: Text::GitlabRights,
    warning: None,
    help: "https://docs.gitlab.com/user/profile/personal_access_tokens/",
};

const PAGE: usize = 20;
const RUNNING: [&str; 5] = ["running", "pending", "preparing", "created", "waiting_for_resource"];

fn list(reply: &Reply) -> Result<Vec<serde_json::Value>, Failure> {
    reply.json()?.as_array().cloned().ok_or(Failure::BadAnswer)
}

impl Service for GitLab {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        let page = PAGE.to_string();
        let mrs = |scope: &str| {
            let query = [("scope", scope), ("state", "opened"), ("order_by", "updated_at"), ("per_page", page.as_str())];
            join(ctx.base, "/api/v4/merge_requests", &query)
        };
        [mrs("created_by_me"), mrs("reviews_for_me"), join(ctx.base, "/api/v4/pipelines", &[("per_page", page.as_str())])]
            .into_iter()
            .map(|url| Request::get(url).header("PRIVATE-TOKEN", ctx.token))
            .collect()
    }

    fn summarize(&self, _: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [mine, reviews, pipelines] = answers else { return Err(Failure::BadAnswer) };
        let (mine, reviews) = (list(mine)?, list(reviews)?);
        // Optional: instances older than 19.3 have no `GET /pipelines`.
        let pipelines = pipelines.json_opt().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let running = pipelines.iter().filter(|p| RUNNING.contains(&text(p, "status"))).count();
        let last_failed = pipelines.first().is_some_and(|p| text(p, "status") == "failed");
        let failed = pipelines.iter().filter(|p| text(p, "status") == "failed").count();
        let open = mine.len() as u64;
        let to_review = reviews.len() as u64;
        let tone = if last_failed {
            Tone::Err
        } else if to_review > 0 {
            Tone::Warn
        } else if running > 0 {
            Tone::Busy
        } else if open > 0 {
            Tone::Ok
        } else {
            Tone::Idle
        };
        let mut parts = vec![tr(if open == 0 { Text::GitlabNoMr } else { Text::GitlabOpenMrs { n: open } })];
        if to_review > 0 {
            parts.push(tr(Text::ReviewsRequested { n: to_review }));
        }
        if last_failed {
            parts.push(tr(Text::GitlabPipelineFailed));
        } else if running > 0 {
            parts.push(tr(Text::GitlabPipelinesRunning { n: running as u64 }));
        }
        let items = mine
            .iter()
            .take(3)
            .map(|mr| {
                let reference = mr["references"]["full"].as_str().unwrap_or("");
                Item::new(text(mr, "title"), reference, parse_time(text(mr, "updated_at")))
            })
            .collect();
        Ok(Summary {
            tone,
            line: parts.join(", "),
            short: tr(if to_review > 0 { Text::ToReview { n: to_review } } else { Text::GitlabShort { n: open } }),
            metrics: vec![
                Metric::new(&tr(Text::GitlabOpenMetric), count_label(mine.len(), mine.len() >= PAGE)),
                Metric::toned(&tr(Text::MetricToReview), to_review, if to_review > 0 { Tone::Warn } else { Tone::Idle }),
                Metric::toned(&tr(Text::GitlabPipelinesMetric), failed, if failed > 0 { Tone::Err } else { Tone::Ok }),
            ],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    /// Fields of the documented list answer (iid, title, state, web_url, references, updated_at).
    const MRS: &str = r#"[{"id":1,"iid":12,"title":"Ajoute la pastille","state":"opened","draft":false,
        "web_url":"https://gitlab.example.com/a/b/-/merge_requests/12","references":{"full":"a/b!12"},
        "updated_at":"2026-10-11T10:00:00.000Z"}]"#;
    /// Documented pipeline fields (id, status, ref, web_url, created_at, updated_at).
    const PIPELINES: &str = r#"[{"id":47,"iid":12,"project_id":1,"status":"running","source":"push","ref":"main",
        "sha":"a91957a858320c0e17f3a0eca7cfacbff50ea29a","web_url":"https://example.com/foo/bar/pipelines/47",
        "created_at":"2016-08-11T11:28:34.085Z","updated_at":"2016-08-11T11:32:35.169Z"},
        {"id":46,"status":"failed","ref":"main"}]"#;

    #[test]
    fn merge_requests_reviews_and_pipelines_on_the_users_instance() {
        let asked = GitLab.plan(&ctx("https://gitlab.example.com", ""), &[]);
        assert_eq!(asked.len(), 3);
        assert!(asked[0].url.starts_with("https://gitlab.example.com/api/v4/merge_requests?scope=created_by_me&state=opened"));
        assert!(asked[1].url.contains("scope=reviews_for_me"));
        assert!(asked[2].url.starts_with("https://gitlab.example.com/api/v4/pipelines?"));
        assert!(asked.iter().all(|r| r.headers == [("PRIVATE-TOKEN", "jeton-de-test".to_owned())]));

        let script = Script::new(vec![ok(MRS), ok("[]"), ok(PIPELINES)]);
        let summary = run(&GitLab, &script, &ctx("https://gitlab.example.com", "")).unwrap();
        assert_eq!(summary.tone, Tone::Busy);
        assert_eq!(summary.line, "1 MR ouverte, 1 pipeline en cours");
        assert_eq!(summary.items[0].detail, "a/b!12");
        assert_eq!(summary.metrics[2].value, "1");
    }

    #[test]
    fn an_older_instance_without_the_pipelines_list_still_reads() {
        let script = Script::new(vec![ok("[]"), ok(MRS), status(404)]);
        let summary = run(&GitLab, &script, &ctx("https://gitlab.example.com", "")).unwrap();
        assert_eq!(summary.tone, Tone::Warn);
        assert_eq!(summary.short, "1 à relire");
        assert_eq!(summary.line, "Aucune MR ouverte, 1 relecture demandée");
    }
}
