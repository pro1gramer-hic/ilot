//! Docker Hub: the user's images and the latest tags of the one pushed last. The personal
//! access token is first traded for a 10-minute bearer (`POST /v2/auth/token`, the
//! documented read that needs a body), then the repositories and tags are listed
//! (https://docs.docker.com/reference/api/hub/latest/). No « builds » pastille: automated
//! builds are deprecated and have no API (https://docs.docker.com/docker-hub/repos/manage/builds/).

use super::{Account, AccountRule, Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, parse_time, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use serde_json::json;
use std::time::Duration;

pub struct DockerHub;

static SPEC: Spec = Spec {
    id: "dockerhub",
    name: "Docker Hub",
    palette: "ciel",
    kind: "meche",
    address: Address::Fixed("https://hub.docker.com"),
    account: Some(Account {
        label: Text::DockerUserLabel,
        hint: Text::DockerUserHint,
        rule: AccountRule::Name,
    }),
    refresh: Duration::from_secs(1800),
    token_label: Text::DockerTokenLabel,
    rights: Text::DockerRights,
    warning: None,
    help: "https://app.docker.com/settings/personal-access-tokens",
};

/// A repository name as Docker Hub writes it: lowercase letters, digits, `.`, `_`, `-`.
fn safe_repo(name: &str) -> bool {
    !name.is_empty() && name.len() <= 255 && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}

fn bearer(answers: &[Reply]) -> Option<String> {
    answers.first()?.json().ok()?.get("access_token")?.as_str().map(str::to_owned)
}

fn repositories(reply: &Reply) -> Result<Vec<serde_json::Value>, Failure> {
    reply.json()?["results"].as_array().cloned().ok_or(Failure::BadAnswer)
}

impl Service for DockerHub {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        let namespace = format!("/v2/namespaces/{}/repositories", ctx.account);
        match answers {
            [] if !ctx.account.is_empty() => {
                vec![Request::post(join(ctx.base, "/v2/auth/token", &[]), json!({ "identifier": ctx.account, "secret": ctx.token }))]
            }
            [_] => bearer(answers).map_or_else(Vec::new, |token| {
                let query = [("page_size", "10"), ("ordering", "-last_updated")];
                vec![Request::get(join(ctx.base, &namespace, &query)).bearer(&token)]
            }),
            [_, repos] => {
                let latest = repositories(repos).ok().and_then(|list| list.first().map(|r| text(r, "name").to_owned()));
                match (latest, bearer(answers)) {
                    (Some(repo), Some(token)) if safe_repo(&repo) => {
                        let url = join(ctx.base, &format!("{namespace}/{repo}/tags"), &[("page_size", "5")]);
                        vec![Request::get(url).bearer(&token)]
                    }
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    fn summarize(&self, ctx: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        if ctx.account.is_empty() {
            return Err(Failure::Setup(Text::UsernameMissing));
        }
        if bearer(answers).is_none() {
            return Err(Failure::Refused);
        }
        let repos = answers.get(1).ok_or(Failure::BadAnswer).and_then(repositories)?;
        let tags = answers.get(2).and_then(Reply::json_opt).and_then(|v| v["results"].as_array().cloned()).unwrap_or_default();
        let latest = repos.first().map(|r| text(r, "name").to_owned()).unwrap_or_default();
        let pulls: u64 = repos.iter().filter_map(|r| r["pull_count"].as_u64()).sum();
        let mut items: Vec<Item> = tags
            .iter()
            .take(3)
            .map(|t| {
                let when = parse_time(text(t, "tag_last_pushed")).or_else(|| parse_time(text(t, "last_updated")));
                Item::new(format!("{latest}:{}", text(t, "name")), tr(Text::DockerTagPushed), when)
            })
            .collect();
        if items.is_empty() {
            items = repos
                .iter()
                .take(3)
                .map(|r| {
                    let detail = if r["is_private"].as_bool().unwrap_or(false) { Text::DockerPrivate } else { Text::DockerPublic };
                    Item::new(text(r, "name"), tr(detail), parse_time(text(r, "last_updated")))
                })
                .collect();
        }
        Ok(Summary {
            tone: if repos.is_empty() { Tone::Idle } else { Tone::Ok },
            line: if repos.is_empty() {
                tr(Text::DockerNone)
            } else {
                tr(Text::DockerLatest { n: repos.len() as u64, latest: &latest })
            },
            short: if latest.is_empty() { "Docker Hub".to_owned() } else { latest.clone() },
            metrics: vec![
                Metric::new(&tr(Text::DockerImages), repos.len()),
                Metric::new(&tr(Text::DockerPulls), pulls),
                Metric::new(&tr(Text::DockerRecentTags), tags.len()),
            ],
            items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;
    use crate::pastilles::http::Method;

    /// Fields of the documented answers (latest.yaml): access_token; results[].name,
    /// namespace, is_private, pull_count, last_updated; tags: name, tag_last_pushed.
    const TOKEN: &str = r#"{"access_token":"jwt-court"}"#;
    const REPOS: &str = r#"{"count":2,"results":[
        {"name":"ilot-api","namespace":"acme","is_private":false,"pull_count":120,"star_count":3,"last_updated":"2026-10-11T09:00:00Z"},
        {"name":"site","namespace":"acme","is_private":true,"pull_count":4,"last_updated":"2026-10-01T09:00:00Z"}]}"#;
    const TAGS: &str = r#"{"count":1,"results":[{"name":"1.4.0","full_size":2048,"tag_status":"active",
        "last_updated":"2026-10-11T09:00:00Z","tag_last_pushed":"2026-10-11T09:00:00Z"}]}"#;

    #[test]
    fn the_token_is_traded_then_images_and_tags_are_read() {
        let script = Script::new(vec![ok(TOKEN), ok(REPOS), ok(TAGS)]);
        let summary = run(&DockerHub, &script, &ctx("https://hub.docker.com", "acme")).unwrap();
        let asked = script.asked();
        assert_eq!(asked[0].method, Method::Post);
        assert_eq!(asked[0].url, "https://hub.docker.com/v2/auth/token");
        assert_eq!(asked[1].url, "https://hub.docker.com/v2/namespaces/acme/repositories?page_size=10&ordering=-last_updated");
        assert_eq!(asked[1].headers, [("Authorization", "Bearer jwt-court".to_owned())]);
        assert_eq!(asked[2].url, "https://hub.docker.com/v2/namespaces/acme/repositories/ilot-api/tags?page_size=5");
        assert_eq!(summary.line, "2 images, dernière mise à jour : ilot-api");
        assert_eq!(summary.items[0].title, "ilot-api:1.4.0");
        assert_eq!(summary.metrics[1].value, "124");
    }

    #[test]
    fn a_refused_exchange_is_a_refused_token() {
        let script = Script::new(vec![ok(r#"{"message":"nope"}"#)]);
        assert_eq!(run(&DockerHub, &script, &ctx("https://hub.docker.com", "acme")), Err(Failure::Refused));
        assert_eq!(script.asked().len(), 1);
    }

    #[test]
    fn odd_repository_names_are_never_put_in_a_url() {
        assert!(safe_repo("ilot-api"));
        assert!(!safe_repo("../../v2/users"));
        assert!(!safe_repo("a?b"));
    }
}
