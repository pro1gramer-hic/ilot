//! GitHub: open pull requests, reviews asked of the user, and CI state, in four searches
//! (https://docs.github.com/en/rest/search/search#search-issues-and-pull-requests):
//! `author:@me`, `review-requested:@me`, and `status:failure` / `status:pending` for the
//! CI (https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests).
//! Search allows 30 requests a minute: four every three minutes stay far below.

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, parse_time, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use std::time::Duration;

pub struct GitHub;

static SPEC: Spec = Spec {
    id: "github",
    name: "GitHub",
    palette: "nuit",
    kind: "meche",
    address: Address::Custom { default: "https://api.github.com", required: false },
    account: None,
    refresh: Duration::from_secs(180),
    token_label: Text::GithubTokenLabel,
    rights: Text::GithubRights,
    warning: None,
    help: "https://github.com/settings/personal-access-tokens/new",
};

const SEARCHES: [(&str, &str); 4] = [
    ("is:pr is:open author:@me archived:false", "5"),
    ("is:pr is:open review-requested:@me archived:false", "1"),
    ("is:pr is:open author:@me status:failure", "1"),
    ("is:pr is:open author:@me status:pending", "1"),
];

fn total(reply: &Reply) -> Result<u64, Failure> {
    reply.json()?.get("total_count").and_then(serde_json::Value::as_u64).ok_or(Failure::BadAnswer)
}

/// « owner/repo » from `repository_url` (https://api.github.com/repos/owner/repo).
fn repo_of(url: &str) -> &str {
    url.split_once("/repos/").map_or("", |(_, repo)| repo)
}

impl Service for GitHub {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        SEARCHES
            .iter()
            .map(|(query, per_page)| {
                let sort = [("q", *query), ("sort", "updated"), ("order", "desc"), ("per_page", *per_page)];
                Request::get(join(ctx.base, "/search/issues", &sort))
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .bearer(ctx.token)
            })
            .collect()
    }

    fn summarize(&self, _: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [mine, reviews, failing, pending] = answers else { return Err(Failure::BadAnswer) };
        let open = total(mine)?;
        let (reviews, failing, pending) = (total(reviews)?, total(failing)?, total(pending)?);
        let items = mine.json()?["items"]
            .as_array()
            .map(|list| {
                list.iter()
                    .take(3)
                    .map(|pr| {
                        let draft = pr["draft"].as_bool().unwrap_or(false);
                        let repo = repo_of(text(pr, "repository_url"));
                        let detail = if draft { tr(Text::GithubDraft { repo }) } else { repo.to_owned() };
                        Item::new(text(pr, "title"), detail, parse_time(text(pr, "updated_at")))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let tone = if failing > 0 {
            Tone::Err
        } else if reviews > 0 {
            Tone::Warn
        } else if pending > 0 {
            Tone::Busy
        } else if open > 0 {
            Tone::Ok
        } else {
            Tone::Idle
        };
        let mut parts = vec![tr(if open == 0 { Text::GithubNoPr } else { Text::GithubOpenPrs { n: open } })];
        if reviews > 0 {
            parts.push(tr(Text::ReviewsRequested { n: reviews }));
        }
        if failing > 0 {
            parts.push(tr(Text::GithubCiFailing { n: failing }));
        }
        Ok(Summary {
            tone,
            line: parts.join(", "),
            short: tr(if reviews > 0 { Text::ToReview { n: reviews } } else { Text::GithubShort { n: open } }),
            metrics: vec![
                Metric::new(&tr(Text::GithubOpenMetric), open),
                Metric::toned(&tr(Text::MetricToReview), reviews, if reviews > 0 { Tone::Warn } else { Tone::Idle }),
                Metric::toned(&tr(Text::GithubCiMetric), failing, if failing > 0 { Tone::Err } else { Tone::Ok }),
            ],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;

    /// Shape of the documented search answer (total_count, incomplete_results, items).
    const MINE: &str = r#"{"total_count":3,"incomplete_results":false,"items":[
        {"title":"Pastilles de services","html_url":"https://github.com/o/ilot/pull/42",
         "repository_url":"https://api.github.com/repos/o/ilot","updated_at":"2026-10-11T11:00:00Z","draft":false,
         "pull_request":{"url":"https://api.github.com/repos/o/ilot/pulls/42","merged_at":null}},
        {"title":"Essai","repository_url":"https://api.github.com/repos/o/lab","updated_at":"2026-10-10T08:00:00Z","draft":true}]}"#;

    #[test]
    fn four_searches_with_the_documented_headers() {
        let asked = GitHub.plan(&ctx("https://api.github.com", ""), &[]);
        assert_eq!(asked.len(), 4);
        assert!(asked[0].url.starts_with("https://api.github.com/search/issues?q=is%3Apr+is%3Aopen+author%3A%40me"));
        assert!(asked[1].url.contains("review-requested%3A%40me"));
        assert!(asked[2].url.contains("status%3Afailure"));
        assert!(asked.iter().all(|r| r.headers.contains(&("X-GitHub-Api-Version", "2022-11-28".to_owned()))));
        assert!(asked.iter().all(|r| r.headers.contains(&("Authorization", "Bearer jeton-de-test".to_owned()))));
        // GitHub Enterprise Server: the same paths under /api/v3.
        let ghes = GitHub.plan(&ctx("https://ghe.example.com/api/v3", ""), &[]);
        assert!(ghes[0].url.starts_with("https://ghe.example.com/api/v3/search/issues?"));
    }

    #[test]
    fn pull_requests_reviews_and_ci() {
        let script = Script::new(vec![
            ok(MINE),
            ok(r#"{"total_count":1,"items":[]}"#),
            ok(r#"{"total_count":1,"items":[]}"#),
            ok(r#"{"total_count":0,"items":[]}"#),
        ]);
        let summary = run(&GitHub, &script, &ctx("https://api.github.com", "")).unwrap();
        assert_eq!(summary.tone, Tone::Err);
        assert_eq!(summary.line, "3 PR ouvertes, 1 relecture demandée, CI en échec sur 1");
        assert_eq!(summary.short, "1 à relire");
        assert_eq!(summary.items[0].title, "Pastilles de services");
        assert_eq!(summary.items[0].detail, "o/ilot");
        assert_eq!(summary.items[1].detail, "o/lab, brouillon");
        assert_eq!(summary.items[0].at, Some(NOW - 3_600_000));
    }

    #[test]
    fn nothing_open_is_calm() {
        let empty = r#"{"total_count":0,"incomplete_results":false,"items":[]}"#;
        let script = Script::new(vec![ok(empty), ok(empty), ok(empty), ok(empty)]);
        let summary = run(&GitHub, &script, &ctx("https://api.github.com", "")).unwrap();
        assert_eq!((summary.tone, summary.line.as_str(), summary.short.as_str()), (Tone::Idle, "Aucune PR ouverte", "0 PR"));
    }

    #[test]
    fn an_unexpected_answer_is_said_so() {
        let script = Script::new(vec![ok("[]"), ok("{}"), ok("{}"), ok("{}")]);
        assert_eq!(run(&GitHub, &script, &ctx("https://api.github.com", "")), Err(Failure::BadAnswer));
    }

    #[test]
    fn the_summary_speaks_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let script = Script::new(vec![
            ok(MINE),
            ok(r#"{"total_count":1,"items":[]}"#),
            ok(r#"{"total_count":1,"items":[]}"#),
            ok(r#"{"total_count":0,"items":[]}"#),
        ]);
        let summary = run(&GitHub, &script, &ctx("https://api.github.com", "")).unwrap();
        assert_eq!(summary.line, "3 open PRs, 1 review requested, CI failing on 1");
        assert_eq!(summary.short, "1 to review");
        assert_eq!(summary.items[1].detail, "o/lab, draft");
    }
}
