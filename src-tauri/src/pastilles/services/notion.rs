//! Notion: the pages edited last, from the search endpoint sorted by
//! `last_edited_time` (https://developers.notion.com/reference/post-search): a POST that
//! only reads. Only pages shared with the connection show. An internal connection with
//! the « Read content » capability alone (https://developers.notion.com/reference/capabilities).
//! Limit: 3 requests a second on average (https://developers.notion.com/reference/request-limits).

use super::{Address, Ctx, Item, Metric, Service, Spec, Summary, Tone, parse_time, text};
use crate::locale::{Text, tr};
use crate::pastilles::http::{Failure, Reply, Request, join};
use serde_json::{Value, json};
use std::time::Duration;

pub struct Notion;

static SPEC: Spec = Spec {
    id: "notion",
    name: "Notion",
    palette: "citron",
    kind: "meche",
    address: Address::Fixed("https://api.notion.com"),
    account: None,
    refresh: Duration::from_secs(300),
    token_label: Text::NotionTokenLabel,
    rights: Text::NotionRights,
    warning: None,
    help: "https://www.notion.so/profile/integrations",
};

/// The version the requests are written for (https://developers.notion.com/reference/versioning).
const VERSION: &str = "2026-03-11";

/// The page's title: its property of type `title`, as plain text.
fn title_of(page: &Value) -> String {
    page["properties"]
        .as_object()
        .and_then(|props| props.values().find(|p| text(p, "type") == "title"))
        .and_then(|p| p["title"].as_array())
        .map(|parts| parts.iter().map(|part| text(part, "plain_text")).collect::<String>())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| tr(Text::NotionUntitled))
}

impl Service for Notion {
    fn spec(&self) -> &'static Spec {
        &SPEC
    }

    fn plan(&self, ctx: &Ctx, answers: &[Reply]) -> Vec<Request> {
        if !answers.is_empty() {
            return Vec::new();
        }
        let body = json!({
            "filter": { "property": "object", "value": "page" },
            "sort": { "direction": "descending", "timestamp": "last_edited_time" },
            "page_size": 5
        });
        vec![Request::post(join(ctx.base, "/v1/search", &[]), body).bearer(ctx.token).header("Notion-Version", VERSION)]
    }

    fn summarize(&self, ctx: &Ctx, answers: &[Reply]) -> Result<Summary, Failure> {
        let [reply] = answers else { return Err(Failure::BadAnswer) };
        let json = reply.json()?;
        let pages = json["results"].as_array().ok_or(Failure::BadAnswer)?;
        let edited = |p: &Value| parse_time(text(p, "last_edited_time"));
        let today = pages.iter().filter(|p| edited(p).is_some_and(|at| at >= ctx.midnight_ms())).count();
        let latest = pages.first().map(title_of);
        Ok(Summary {
            tone: if pages.is_empty() { Tone::Idle } else { Tone::Ok },
            line: match &latest {
                None => tr(Text::NotionNone),
                Some(title) => tr(Text::NotionLatest { title }),
            },
            short: latest.clone().unwrap_or_else(|| "Notion".to_owned()),
            metrics: vec![Metric::new(&tr(Text::NotionEditedToday), today), Metric::new(&tr(Text::NotionRecent), pages.len())],
            items: pages.iter().take(3).map(|p| Item::new(title_of(p), tr(Text::NotionPageEdited), edited(p))).collect(),
        })
        .map(|mut summary| {
            if today > 0 {
                summary.line = tr(Text::NotionToday { n: today as u64, line: &summary.line });
            }
            summary
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;
    use crate::pastilles::http::Method;

    /// The documented search answer (object, results[] of pages with properties,
    /// last_edited_time, url; has_more, next_cursor).
    const RESULTS: &str = r#"{"object":"list","results":[
        {"object":"page","id":"954b67f9-3f87-41db-8874-23b92bbd31ee","last_edited_time":"2026-10-11T11:42:00.000Z",
         "url":"https://www.notion.so/Plan-954b67f93f8741db887423b92bbd31ee",
         "properties":{"title":{"id":"title","type":"title","title":[{"type":"text","plain_text":"Plan "},{"plain_text":"de la semaine"}]}}},
        {"object":"page","last_edited_time":"2026-10-08T09:00:00.000Z","properties":{"Nom":{"type":"title","title":[]}}}],
        "next_cursor":null,"has_more":false,"type":"page_or_database","page_or_database":{}}"#;

    #[test]
    fn recent_pages_through_a_read_only_search() {
        let asked = Notion.plan(&ctx("https://api.notion.com", ""), &[]);
        assert_eq!(asked[0].method, Method::Post);
        assert_eq!(asked[0].url, "https://api.notion.com/v1/search");
        assert!(asked[0].headers.contains(&("Notion-Version", VERSION.to_owned())));
        assert_eq!(asked[0].body.as_ref().unwrap()["sort"]["timestamp"], "last_edited_time");
        let script = Script::new(vec![ok(RESULTS)]);
        let summary = run(&Notion, &script, &ctx("https://api.notion.com", "")).unwrap();
        assert_eq!(summary.line, "1 page modifiée aujourd'hui. Dernière modifiée : Plan de la semaine");
        assert_eq!(summary.items[1].title, "Sans titre");
    }
}
