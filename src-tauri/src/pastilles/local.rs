//! Local model servers, on this machine only (127.0.0.1): Ollama
//! (https://docs.ollama.com/api/ps, https://docs.ollama.com/api/tags) and LM Studio
//! (https://lmstudio.ai/docs/developer/rest/list). They tell which models are installed
//! and which sit in memory; neither tells whether a model answers another program
//! (https://docs.ollama.com/faq), so « Mochi réfléchit » belongs to îlot's own chat.
//! A model download (https://docs.ollama.com/api/pull) only starts on the user's click,
//! and only its own progress can be seen: Ollama shows no one else's.

use super::http::{Failure, Reply, Request};
use super::services::{Tone, clip, parse_time, text};
use crate::locale::{Text, tr};
use serde::Serialize;
use std::collections::BTreeMap;

pub const OLLAMA: &str = "http://127.0.0.1:11434";
pub const LMSTUDIO: &str = "http://127.0.0.1:1234";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Server {
    Ollama,
    LmStudio,
}

impl Server {
    pub fn id(self) -> &'static str {
        match self {
            Server::Ollama => "ollama",
            Server::LmStudio => "lmstudio",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Server::Ollama => "Ollama",
            Server::LmStudio => "LM Studio",
        }
    }

    pub fn requests(self) -> Vec<Request> {
        match self {
            Server::Ollama => vec![Request::get(format!("{OLLAMA}/api/ps")), Request::get(format!("{OLLAMA}/api/tags"))],
            Server::LmStudio => vec![Request::get(format!("{LMSTUDIO}/api/v1/models"))],
        }
    }

    pub fn read(self, answers: &[Reply]) -> Result<Vec<LocalModel>, Failure> {
        match (self, answers) {
            (Server::Ollama, [ps, tags]) => ollama_models(ps, tags),
            (Server::LmStudio, [models]) => lmstudio_models(models),
            _ => Err(Failure::BadAnswer),
        }
    }
}

/// A model of a local server: installed, and maybe loaded in memory.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModel {
    pub name: String,
    /// Bytes on disk (Ollama) or of the file (LM Studio).
    pub size: u64,
    pub loaded: bool,
    /// Graphics memory in use, when Ollama tells it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram: Option<u64>,
    /// When Ollama plans to unload it (epoch ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<i64>,
    /// « 8.0B », « Q4_K_M »…
    pub details: String,
}

fn ollama_models(ps: &Reply, tags: &Reply) -> Result<Vec<LocalModel>, Failure> {
    let loaded = ps.json()?["models"].as_array().cloned().ok_or(Failure::BadAnswer)?;
    let installed = tags.json()?["models"].as_array().cloned().ok_or(Failure::BadAnswer)?;
    let details = |m: &serde_json::Value| {
        let d = &m["details"];
        [text(d, "parameter_size"), text(d, "quantization_level")].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(", ")
    };
    let mut models: Vec<LocalModel> = loaded
        .iter()
        .map(|m| LocalModel {
            name: text(m, "name").to_owned(),
            size: m["size"].as_u64().unwrap_or(0),
            loaded: true,
            vram: m["size_vram"].as_u64(),
            until: parse_time(text(m, "expires_at")),
            details: details(m),
        })
        .collect();
    for m in &installed {
        let name = text(m, "name");
        if !models.iter().any(|known| known.name == name) {
            models.push(LocalModel {
                name: name.to_owned(),
                size: m["size"].as_u64().unwrap_or(0),
                loaded: false,
                vram: None,
                until: None,
                details: details(m),
            });
        }
    }
    models.retain(|m| !m.name.is_empty());
    Ok(models)
}

fn lmstudio_models(reply: &Reply) -> Result<Vec<LocalModel>, Failure> {
    let list = reply.json()?["models"].as_array().cloned().ok_or(Failure::BadAnswer)?;
    let mut models: Vec<LocalModel> = list
        .iter()
        // Embedding models cannot chat.
        .filter(|m| text(m, "type") == "llm")
        .map(|m| LocalModel {
            name: text(m, "key").to_owned(),
            size: m["size_bytes"].as_u64().unwrap_or(0),
            loaded: m["loaded_instances"].as_array().is_some_and(|l| !l.is_empty()),
            vram: None,
            until: None,
            details: text(m, "display_name").to_owned(),
        })
        .filter(|m| !m.name.is_empty())
        .collect();
    models.sort_by_key(|m| !m.loaded);
    Ok(models)
}

/// Tone, line and pill words of a local server's pastille.
pub fn describe(server: Server, models: &Result<Vec<LocalModel>, Failure>) -> (Tone, String, String) {
    let name = server.name();
    match models {
        Err(Failure::Unreachable) => (Tone::Off, tr(Text::LocalNotRunning { name }), name.to_owned()),
        Err(Failure::Refused) => (Tone::Err, tr(Text::LocalNeedsToken { name }), name.to_owned()),
        Err(_) => (Tone::Err, tr(Text::LocalOdd { name }), name.to_owned()),
        Ok(models) => {
            let loaded: Vec<&LocalModel> = models.iter().filter(|m| m.loaded).collect();
            match loaded.as_slice() {
                [] if models.is_empty() => (Tone::Idle, tr(Text::NoModelInstalled), name.to_owned()),
                [] => (Tone::Idle, tr(Text::NoModelLoaded { n: models.len() as u64 }), name.to_owned()),
                [one] => (Tone::Busy, tr(Text::ModelLoaded { name: &one.name }), clip(&one.name, 18)),
                many => (Tone::Busy, tr(Text::ModelsLoaded { n: many.len() }), clip(&many[0].name, 18)),
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// A download started from îlot

/// A model name the user typed: « gemma4 », « llama3.2:3b », « hf.co/user/model:Q4_K_M ».
pub fn valid_model(name: &str) -> bool {
    (1..=100).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
        && name.as_bytes()[0].is_ascii_alphanumeric()
        && !name.contains("..")
        && !name.contains("//")
}

/// Where a download stands, as the pastille shows it.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pull {
    pub model: String,
    /// « Préparation », « Téléchargement », « Vérification », « Terminé »…
    pub step: String,
    pub completed: u64,
    pub total: u64,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Bytes of each layer: the progress sums them.
    #[serde(skip)]
    layers: BTreeMap<String, (u64, u64)>,
}

impl Pull {
    pub fn new(model: &str) -> Self {
        Self { model: model.to_owned(), step: tr(Text::PullPreparing), ..Self::default() }
    }

    /// Reads one line of the NDJSON stream (status, digest, total, completed, or error).
    pub fn read_line(&mut self, line: &str) {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) else { return };
        if let Some(error) = json["error"].as_str() {
            self.error = Some(clip(error, 120));
            self.done = true;
            return;
        }
        let status = text(&json, "status");
        self.step = tr(match status {
            "success" => Text::PullDone,
            "verifying sha256 digest" => Text::PullVerifying,
            "writing manifest" | "removing any unused layers" => Text::PullFinishing,
            s if s.starts_with("pulling") && json.get("digest").is_some() => Text::PullDownloading,
            _ => Text::PullPreparing,
        });
        if let (Some(digest), Some(total)) = (json["digest"].as_str(), json["total"].as_u64()) {
            let completed = json["completed"].as_u64().unwrap_or(0).min(total);
            self.layers.insert(digest.to_owned(), (completed, total));
            self.completed = self.layers.values().map(|(c, _)| c).sum();
            self.total = self.layers.values().map(|(_, t)| t).sum();
        }
        if status == "success" {
            self.done = true;
            self.completed = self.total;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(json: &str) -> Reply {
        Reply { status: 200, body: json.as_bytes().to_vec(), retry_after: None }
    }

    /// The documented examples of /api/ps and /api/tags.
    const PS: &str = r#"{"models":[{"name":"gemma4","model":"gemma4","size":6591830464,"size_vram":5333539264,
        "expires_at":"2025-10-17T16:47:07.93355-07:00","context_length":4096,
        "details":{"family":"gemma4","parameter_size":"8.0B","quantization_level":"Q4_K_M"}}]}"#;
    const TAGS: &str = r#"{"models":[{"name":"gemma4","modified_at":"2025-10-03T23:34:03.409490317-07:00","size":9608350245,
        "details":{"format":"gguf","parameter_size":"8.0B","quantization_level":"Q4_K_M"}},
        {"name":"llama3.2:3b","size":2019393189,"details":{"parameter_size":"3.2B"}}]}"#;
    /// The documented example of LM Studio's GET /api/v1/models.
    const LMS: &str = r#"{"models":[
        {"type":"llm","key":"google/gemma-4-26b-a4b","display_name":"Gemma 4 26B A4B","size_bytes":17990911801,
         "loaded_instances":[{"id":"google/gemma-4-26b-a4b","config":{"context_length":4096}}],"max_context_length":262144},
        {"type":"embedding","key":"text-embedding-nomic-embed-text-v1.5-embedding","loaded_instances":[]},
        {"type":"llm","key":"qwen/qwen3-8b","display_name":"Qwen3 8B","loaded_instances":[]}]}"#;

    #[test]
    fn ollama_lists_loaded_then_installed_models() {
        let models = Server::Ollama.read(&[ok(PS), ok(TAGS)]).unwrap();
        assert_eq!(models.len(), 2);
        assert!(models[0].loaded && models[0].name == "gemma4");
        assert_eq!(models[0].vram, Some(5_333_539_264));
        assert_eq!(models[0].details, "8.0B, Q4_K_M");
        assert!(models[0].until.is_some());
        assert!(!models[1].loaded);
        let (tone, line, short) = describe(Server::Ollama, &Ok(models));
        assert_eq!((tone, line.as_str(), short.as_str()), (Tone::Busy, "gemma4 en mémoire", "gemma4"));
    }

    #[test]
    fn only_localhost_is_asked() {
        for server in [Server::Ollama, Server::LmStudio] {
            assert!(server.requests().iter().all(|r| r.url.starts_with("http://127.0.0.1:") && r.headers.is_empty()));
        }
    }

    #[test]
    fn lm_studio_models_that_can_chat() {
        let models = Server::LmStudio.read(&[ok(LMS)]).unwrap();
        assert_eq!(models.len(), 2);
        assert!(models[0].loaded);
        assert_eq!(models[0].details, "Gemma 4 26B A4B");
        assert!(!models[1].loaded);
    }

    #[test]
    fn a_server_that_is_not_started_is_said_calmly() {
        let (tone, line, _) = describe(Server::Ollama, &Err(Failure::Unreachable));
        assert_eq!((tone, line.as_str()), (Tone::Off, "Ollama n'est pas lancé"));
        let (tone, line, _) = describe(Server::LmStudio, &Ok(vec![]));
        assert_eq!((tone, line.as_str()), (Tone::Idle, "Aucun modèle installé"));
    }

    #[test]
    fn model_names_typed_by_the_user_are_checked() {
        assert!(valid_model("gemma4"));
        assert!(valid_model("llama3.2:3b"));
        assert!(valid_model("hf.co/bartowski/Llama-3.2-1B-Instruct-GGUF:Q4_K_M"));
        assert!(!valid_model(""));
        assert!(!valid_model("-rm"));
        assert!(!valid_model("../../etc"));
        assert!(!valid_model("a b"));
        assert!(!valid_model("x;rm -rf"));
        assert!(!valid_model(&"a".repeat(101)));
    }

    #[test]
    fn download_progress_sums_the_layers() {
        // The documented stream of POST /api/pull.
        let mut pull = Pull::new("gemma4");
        pull.read_line(r#"{"status":"pulling manifest"}"#);
        assert_eq!(pull.step, "Préparation");
        pull.read_line(r#"{"status":"pulling 6a0746a1ec1a","digest":"sha256:6a0746a1ec1a","total":2000,"completed":500}"#);
        pull.read_line(r#"{"status":"pulling 4fa551d4f938","digest":"sha256:4fa551d4f938","total":2000,"completed":1500}"#);
        assert_eq!((pull.step.as_str(), pull.completed, pull.total), ("Téléchargement", 2000, 4000));
        pull.read_line(r#"{"status":"verifying sha256 digest"}"#);
        pull.read_line("pas du json");
        pull.read_line(r#"{"status":"success"}"#);
        assert!(pull.done && pull.error.is_none());
        assert_eq!((pull.step.as_str(), pull.completed), ("Terminé", 4000));
        let json = serde_json::to_value(&pull).unwrap();
        assert!(json.get("layers").is_none());
    }

    #[test]
    fn a_failed_download_says_why() {
        let mut pull = Pull::new("nope");
        pull.read_line(r#"{"error":"pull model manifest: file does not exist"}"#);
        assert!(pull.done);
        assert_eq!(pull.error.as_deref(), Some("pull model manifest: file does not exist"));
    }

    #[test]
    fn local_servers_are_described_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(describe(Server::Ollama, &Err(Failure::Unreachable)).1, "Ollama isn't running");
        assert_eq!(describe(Server::LmStudio, &Ok(Vec::new())).1, "No model installed");
        assert_eq!(Pull::new("gemma4").step, "Preparing");
    }
}
