//! What the user chose in Réglages › Pastilles (part of settings.json): the main tool and
//! up to four more pastilles. Never a token here: tokens live in the vault (vault.rs).
//! Nothing is on by default.

use super::http::base_url;
use super::local::Server;
use super::services::{self, Address};
use super::tools;
use serde::{Deserialize, Serialize};

/// Pastilles besides the main tool.
pub const MAX_EXTRAS: usize = 4;
/// Pastilles that need no token: local servers, and Spotify (read by media.rs).
pub const LOCAL: [&str; 3] = ["ollama", "lmstudio", "spotify"];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Extra {
    /// « ollama », « lmstudio », « spotify », or a service id (« github »…).
    pub id: String,
    /// The user's instance, for the services that take one; empty means the default.
    pub base_url: String,
    /// The identifier a service needs besides its token (Cloudflare's account id…).
    pub account: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PastilleSettings {
    /// « vscode », « cursor », « codex », « antigravity », or empty (none, the default).
    pub tool: String,
    pub extras: Vec<Extra>,
}

impl Extra {
    /// The pastille as it may be kept, or nothing when it cannot work as given.
    fn sanitized(self) -> Option<Extra> {
        if LOCAL.contains(&self.id.as_str()) {
            return Some(Extra { id: self.id, ..Extra::default() });
        }
        let spec = services::find(&self.id)?.spec();
        let base_url = match spec.address {
            Address::Fixed(_) => String::new(),
            Address::Custom { required, .. } => match base_url(&self.base_url) {
                Ok(url) => url,
                Err(_) if !required && self.base_url.trim().is_empty() => String::new(),
                Err(_) => return None,
            },
        };
        let account = match spec.account {
            None => String::new(),
            Some(account) => account.rule.clean(&self.account).ok()?,
        };
        Some(Extra { id: self.id, base_url, account })
    }
}

impl PastilleSettings {
    /// Unknown tools and pastilles are dropped, each one is kept once, four at most, and
    /// every URL and identifier is checked.
    pub fn sanitized(self) -> Self {
        let tool = if tools::find(&self.tool).is_some() { self.tool } else { String::new() };
        let mut extras: Vec<Extra> = Vec::new();
        for extra in self.extras.into_iter().filter_map(Extra::sanitized) {
            if extras.len() < MAX_EXTRAS && !extras.iter().any(|e| e.id == extra.id) {
                extras.push(extra);
            }
        }
        Self { tool, extras }
    }
}

/// One thing the core keeps reading while its pastille is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Job {
    Tool(&'static str),
    Local(Server),
    Service { id: &'static str, base: String, account: String },
}

impl Job {
    pub fn id(&self) -> &'static str {
        match self {
            Job::Tool(_) => "tool",
            Job::Local(server) => server.id(),
            Job::Service { id, .. } => id,
        }
    }
}

/// What the core must read for these settings: nothing at all by default.
pub fn jobs(settings: &PastilleSettings) -> Vec<Job> {
    let mut jobs = Vec::new();
    if let Some(tool) = tools::find(&settings.tool) {
        jobs.push(Job::Tool(tool.id));
    }
    for extra in &settings.extras {
        match extra.id.as_str() {
            "ollama" => jobs.push(Job::Local(Server::Ollama)),
            "lmstudio" => jobs.push(Job::Local(Server::LmStudio)),
            // Spotify comes from the media sessions, already followed by media.rs.
            "spotify" => {}
            id => {
                if let Some(service) = services::find(id) {
                    let spec = service.spec();
                    let base = match spec.address {
                        Address::Fixed(url) => url.to_owned(),
                        Address::Custom { default, .. } if extra.base_url.is_empty() => default.to_owned(),
                        Address::Custom { .. } => extra.base_url.clone(),
                    };
                    jobs.push(Job::Service { id: spec.id, base, account: extra.account.clone() });
                }
            }
        }
    }
    jobs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extra(id: &str, base_url: &str, account: &str) -> Extra {
        Extra { id: id.into(), base_url: base_url.into(), account: account.into() }
    }

    #[test]
    fn nothing_is_on_by_default() {
        let settings = PastilleSettings::default();
        assert_eq!(settings.tool, "");
        assert!(settings.extras.is_empty());
        assert!(jobs(&settings).is_empty());
        // An older settings file has no pastilles at all.
        let old: PastilleSettings = serde_json::from_str("{}").unwrap();
        assert!(jobs(&old.sanitized()).is_empty());
    }

    #[test]
    fn four_extras_at_most_each_once() {
        let settings = PastilleSettings {
            tool: "vscode".into(),
            extras: vec![
                extra("ollama", "", ""),
                extra("ollama", "", ""),
                extra("spotify", "https://evil.example", "x"),
                extra("stripe", "", ""),
                extra("notion", "", ""),
                extra("vercel", "", ""),
                extra("inconnu", "", ""),
            ],
        }
        .sanitized();
        let ids: Vec<&str> = settings.extras.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["ollama", "spotify", "stripe", "notion"]);
        assert_eq!(settings.extras[1], extra("spotify", "", ""));
        let jobs = jobs(&settings);
        assert_eq!(jobs.len(), 4, "the tool, Ollama, Stripe and Notion; Spotify needs no job");
        assert_eq!(jobs[0], Job::Tool("vscode"));
    }

    #[test]
    fn unknown_tools_are_dropped() {
        assert_eq!(PastilleSettings { tool: "notepad".into(), extras: vec![] }.sanitized().tool, "");
    }

    #[test]
    fn custom_addresses_are_checked() {
        let clean = |e: Extra| PastilleSettings { tool: String::new(), extras: vec![e] }.sanitized().extras;
        assert_eq!(clean(extra("dokploy", "https://deploy.example.com/", ""))[0].base_url, "https://deploy.example.com");
        assert!(clean(extra("dokploy", "", "")).is_empty(), "Dokploy needs its instance");
        assert!(clean(extra("n8n", "http://n8n.example.com", "")).is_empty(), "plain http only on localhost");
        assert_eq!(clean(extra("n8n", "http://localhost:5678", ""))[0].base_url, "http://localhost:5678");
        assert_eq!(clean(extra("gitlab", "", ""))[0].base_url, "", "gitlab.com by default");
        assert!(clean(extra("gitlab", "pas une url", "")).is_empty());
        assert_eq!(clean(extra("stripe", "https://ailleurs.example", ""))[0].base_url, "", "fixed address");
        let gitlab = jobs(&PastilleSettings { tool: String::new(), extras: clean(extra("gitlab", "", "")) });
        assert_eq!(gitlab[0], Job::Service { id: "gitlab", base: "https://gitlab.com".into(), account: String::new() });
    }

    #[test]
    fn identifiers_are_checked() {
        let clean = |e: Extra| PastilleSettings { tool: String::new(), extras: vec![e] }.sanitized().extras;
        assert!(clean(extra("cloudflare", "", "pas-un-id")).is_empty());
        assert_eq!(clean(extra("cloudflare", "", "023E105F4ECEF8AD9CA31A8372D0C353"))[0].account, "023e105f4ecef8ad9ca31a8372d0c353");
        assert_eq!(clean(extra("dockerhub", "", "Acme"))[0].account, "acme");
        assert!(clean(extra("dockerhub", "", "a/b")).is_empty());
    }

    #[test]
    fn no_secret_field_exists() {
        let json = serde_json::to_string(&PastilleSettings { tool: "cursor".into(), extras: vec![extra("github", "", "")] }).unwrap();
        assert_eq!(json, r#"{"tool":"cursor","extras":[{"id":"github","baseUrl":"","account":""}]}"#);
    }
}
