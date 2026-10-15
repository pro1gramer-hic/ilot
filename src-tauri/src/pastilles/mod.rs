//! Pastilles: small blocks of state in the island (the main tool, local models, services
//! the user added). The core reads them, one task per active pastille, and tells the
//! island (`pastilles:changed`). No pastille, no task, no request: nothing is on by
//! default. What is read stays in memory, never on disk; tokens stay in the vault.

pub mod config;
pub mod http;
pub mod local;
pub mod services;
pub mod tools;

use crate::locale::{Text, tr};
use crate::window::WINDOW_LABEL;
use config::{Job, PastilleSettings, jobs};
use http::{Failure, Http, Transport};
use local::{LocalModel, Pull, Server};
use serde::Serialize;
use services::{Address, Ctx, Item, Metric, Summary, Tone};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Notify;
use tools::ToolView;

pub const EVENT_CHANGED: &str = "pastilles:changed";

/// The main tool's window is looked at this often (EnumWindows, no network).
const TOOL_EVERY: Duration = Duration::from_secs(4);
/// A local server is asked this often while it runs, and less and less once it stops.
const LOCAL_EVERY: Duration = Duration::from_secs(15);
const LOCAL_MAX: Duration = Duration::from_secs(60);
/// After repeated failures, a service waits at most this long before trying again.
const SERVICE_MAX: Duration = Duration::from_secs(30 * 60);
/// A limited service asking for more is still asked again within the hour.
const LIMIT_MAX: Duration = Duration::from_secs(60 * 60);
/// « Réessayer » never asks a service twice within this time.
const MIN_GAP: Duration = Duration::from_secs(30);
/// Download progress is told at most this often.
const PULL_EVERY: Duration = Duration::from_millis(250);
/// A large model takes a while: the download's own time limit.
const PULL_TIMEOUT: Duration = Duration::from_secs(6 * 3600);

/// One pastille as the island shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PastilleView {
    /// « tool », « ollama », « lmstudio » or a service id.
    pub id: String,
    pub tone: Tone,
    pub line: String,
    /// A few words for the folded pill.
    pub short: String,
    pub metrics: Vec<Metric>,
    pub items: Vec<Item>,
    /// The last reading's problem, told calmly (the last good data may stay below it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    /// Epoch milliseconds of the last reading; none before the first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<LocalModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull: Option<Pull>,
}

impl PastilleView {
    fn waiting(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            tone: Tone::Idle,
            line: tr(Text::PastilleReading),
            short: String::new(),
            metrics: Vec::new(),
            items: Vec::new(),
            problem: None,
            checked_at: None,
            tool: None,
            models: None,
            pull: None,
        }
    }
}

struct Running {
    job: Job,
    task: JoinHandle<()>,
    wake: Arc<Notify>,
}

#[derive(Default)]
struct Inner {
    views: Vec<PastilleView>,
    running: Vec<Running>,
    pull: Option<(Pull, JoinHandle<()>)>,
}

#[derive(Default)]
pub struct PastilleHub {
    inner: Mutex<Inner>,
    http: Mutex<Option<Arc<Http>>>,
}

impl PastilleHub {
    fn http(&self) -> Result<Arc<Http>, Failure> {
        let mut http = self.http.lock().unwrap();
        if http.is_none() {
            *http = Some(Arc::new(Http::new().map_err(|_| Failure::Unreachable)?));
        }
        http.as_ref().map(Arc::clone).ok_or(Failure::Unreachable)
    }

    /// Every view, in the order of the jobs, with the download (if any) on Ollama.
    fn snapshot(&self) -> Vec<PastilleView> {
        let inner = self.inner.lock().unwrap();
        inner
            .views
            .iter()
            .map(|view| {
                let mut view = view.clone();
                if view.id == "ollama" {
                    view.pull = inner.pull.as_ref().map(|(pull, _)| pull.clone());
                }
                view
            })
            .collect()
    }

    /// Replaces a view, when its pastille is still on; true when it changed.
    fn set(&self, next: PastilleView) -> bool {
        let mut inner = self.inner.lock().unwrap();
        match inner.views.iter_mut().find(|v| v.id == next.id) {
            Some(view) if *view != next => {
                *view = next;
                true
            }
            _ => false,
        }
    }

    fn view(&self, id: &str) -> Option<PastilleView> {
        self.inner.lock().unwrap().views.iter().find(|v| v.id == id).cloned()
    }
}

fn emit(app: &AppHandle) {
    let views = app.state::<PastilleHub>().snapshot();
    let _ = app.emit_to(WINDOW_LABEL, EVENT_CHANGED, views);
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Local time minus UTC now, in minutes, daylight saving included
/// (https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-filetimetolocalfiletime).
fn local_offset_min(now_ms: i64) -> i32 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::Storage::FileSystem::FileTimeToLocalFileTime;
    let ticks = (now_ms * 10_000 + 116_444_736_000_000_000) as u64;
    let utc = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
    let mut local = FILETIME::default();
    // SAFETY: two valid FILETIME values on the stack.
    if unsafe { FileTimeToLocalFileTime(&utc, &mut local) }.is_err() {
        return 0;
    }
    let local_ticks = ((local.dwHighDateTime as u64) << 32) | local.dwLowDateTime as u64;
    ((local_ticks as i64 - ticks as i64) / 600_000_000) as i32
}

/// How long a service waits before its next reading: its own pace when all is well,
/// longer after failures (doubling), what a limited service asks for, never less than
/// its pace and never more than the caps above.
pub fn next_wait(refresh: Duration, result: &Result<(), Failure>, failures: u32) -> Duration {
    match result {
        Ok(()) => refresh,
        Err(Failure::Limited(Some(seconds))) => refresh.max(Duration::from_secs(*seconds)).min(LIMIT_MAX.max(refresh)),
        Err(Failure::Limited(None)) => (refresh * 2).min(LIMIT_MAX.max(refresh)),
        Err(Failure::Refused | Failure::NoToken | Failure::Setup(_)) => SERVICE_MAX.max(refresh),
        Err(_) => (refresh * 2u32.saturating_pow(failures.min(6))).min(SERVICE_MAX.max(refresh)),
    }
}

/// A local server's pace: every 15 s while it runs, then 30 s and 60 s once it stopped.
pub fn local_wait(running: bool, failures: u32) -> Duration {
    if running { LOCAL_EVERY } else { (LOCAL_EVERY * 2u32.pow(failures.clamp(1, 2))).min(LOCAL_MAX) }
}

/// Waits `wait`, or less when the user asks to try again (never within `MIN_GAP` of the
/// last reading).
async fn pause(wait: Duration, wake: &Notify, last: Instant) {
    tokio::select! {
        _ = tokio::time::sleep(wait) => {}
        _ = wake.notified() => {
            let since = last.elapsed();
            if since < MIN_GAP {
                tokio::time::sleep(MIN_GAP - since).await;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// The readings

async fn run_tool(app: AppHandle, id: &'static str) {
    let Some(tool) = tools::find(id) else { return };
    loop {
        let look = tauri::async_runtime::spawn_blocking(move || tools::look(tool)).await.unwrap_or_default();
        let line = match (&look.open, &look.project) {
            (false, _) => tr(Text::ToolClosed { tool: tool.name }),
            (true, Some(project)) => tr(Text::ToolOpenOn { tool: tool.name, project }),
            (true, None) => tr(Text::ToolOpen { tool: tool.name }),
        };
        let view = PastilleView {
            tone: if look.open { Tone::Ok } else { Tone::Off },
            short: look.project.clone().unwrap_or_else(|| tool.name.to_owned()),
            line,
            tool: Some(look),
            ..PastilleView::waiting("tool")
        };
        if app.state::<PastilleHub>().set(view) {
            emit(&app);
        }
        tokio::time::sleep(TOOL_EVERY).await;
    }
}

async fn read_local(transport: &dyn Transport, server: Server) -> Result<Vec<LocalModel>, Failure> {
    let mut answers = Vec::new();
    for request in server.requests() {
        let reply = transport.send(request).await?;
        if let Some(failure) = http::failure_of(&reply) {
            return Err(failure);
        }
        answers.push(reply);
    }
    server.read(&answers)
}

async fn run_local(app: AppHandle, server: Server, wake: Arc<Notify>) {
    let mut failures = 0;
    loop {
        let hub = app.state::<PastilleHub>();
        let models = match hub.http() {
            Ok(http) => read_local(http.as_ref(), server).await,
            Err(failure) => Err(failure),
        };
        let (tone, line, short) = local::describe(server, &models);
        let running = models.is_ok();
        failures = if running { 0 } else { failures + 1 };
        let view = PastilleView {
            tone,
            line,
            short,
            models: Some(models.unwrap_or_default()),
            checked_at: Some(now_ms()),
            ..PastilleView::waiting(server.id())
        };
        // The time of the reading alone does not make news.
        let changed = hub.view(server.id()).is_none_or(|old| PastilleView { checked_at: view.checked_at, ..old } != view);
        if hub.set(view) && changed {
            emit(&app);
        }
        pause(local_wait(running, failures), &wake, Instant::now()).await;
    }
}

async fn run_service(app: AppHandle, id: &'static str, base: String, account: String, wake: Arc<Notify>) {
    let Some(service) = services::find(id) else { return };
    let refresh = service.spec().refresh;
    let mut failures = 0;
    loop {
        let started = Instant::now();
        let name = format!("service:{id}");
        let token = tauri::async_runtime::spawn_blocking(move || crate::vault::read(&name)).await;
        let hub = app.state::<PastilleHub>();
        let result: Result<Summary, Failure> = match (token, hub.http()) {
            (Ok(Ok(Some(token))), Ok(http)) => {
                let now = now_ms();
                let ctx = Ctx { base: &base, token: &token, account: &account, now_ms: now, offset_min: local_offset_min(now) };
                services::read(service, http.as_ref(), &ctx).await
            }
            (_, Err(failure)) => Err(failure),
            _ => Err(Failure::NoToken),
        };
        let previous = hub.view(id).unwrap_or_else(|| PastilleView::waiting(id));
        let view = match &result {
            Ok(summary) => PastilleView {
                tone: summary.tone,
                line: summary.line.clone(),
                short: summary.short.clone(),
                metrics: summary.metrics.clone(),
                items: summary.items.clone(),
                problem: None,
                checked_at: Some(now_ms()),
                ..PastilleView::waiting(id)
            },
            // A passing problem keeps the last good numbers under it; a refused token or
            // a missing setting does not.
            Err(failure) => {
                let keep = matches!(failure, Failure::Unreachable | Failure::Limited(_) | Failure::Server(_)) && previous.checked_at.is_some();
                PastilleView {
                    tone: if matches!(failure, Failure::Unreachable) { Tone::Off } else { Tone::Err },
                    problem: Some(failure.line()),
                    checked_at: Some(now_ms()),
                    ..if keep { previous } else { PastilleView { line: failure.line(), ..PastilleView::waiting(id) } }
                }
            }
        };
        if hub.set(view) {
            emit(&app);
        }
        failures = if result.is_ok() { 0 } else { failures + 1 };
        pause(next_wait(refresh, &result.map(|_| ()), failures), &wake, started).await;
    }
}

// ---------------------------------------------------------------------------------------
// Settings → tasks

/// Starts and stops the readings so that exactly the active pastilles run.
pub fn settings_changed(app: &AppHandle, settings: &PastilleSettings) {
    let wanted = jobs(settings);
    let hub = app.state::<PastilleHub>();
    {
        let mut inner = hub.inner.lock().unwrap();
        // A removed pastille stops at once (its task is cancelled mid-request if need be).
        let (keep, stop): (Vec<Running>, Vec<Running>) = inner.running.drain(..).partition(|r| wanted.contains(&r.job));
        for running in stop {
            running.task.abort();
        }
        if !wanted.contains(&Job::Local(Server::Ollama))
            && let Some((_, task)) = inner.pull.take()
        {
            task.abort();
        }
        let mut running = keep;
        for job in &wanted {
            if running.iter().any(|r| r.job == *job) {
                continue;
            }
            let wake = Arc::new(Notify::new());
            let task = match job.clone() {
                Job::Tool(id) => tauri::async_runtime::spawn(run_tool(app.clone(), id)),
                Job::Local(server) => tauri::async_runtime::spawn(run_local(app.clone(), server, wake.clone())),
                Job::Service { id, base, account } => {
                    tauri::async_runtime::spawn(run_service(app.clone(), id, base, account, wake.clone()))
                }
            };
            running.push(Running { job: job.clone(), task, wake });
        }
        // Views follow the order of the jobs; a new one waits for its first reading.
        let views = std::mem::take(&mut inner.views);
        inner.views = wanted
            .iter()
            .map(|job| views.iter().find(|v| v.id == job.id()).cloned().unwrap_or_else(|| PastilleView::waiting(job.id())))
            .collect();
        inner.running = running;
    }
    emit(app);
}

/// Called once at startup, with the saved settings.
pub fn start(app: &AppHandle) {
    let settings = app.state::<crate::settings::SettingsStore>().get();
    settings_changed(app, &settings.pastilles);
}

// ---------------------------------------------------------------------------------------
// What the settings' catalog shows

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    label: String,
    hint: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    id: &'static str,
    name: &'static str,
    palette: &'static str,
    kind: &'static str,
    /// The user's instance: its default, and whether it must be given.
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_url: Option<&'static str>,
    url_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<AccountInfo>,
    refresh_minutes: u64,
    token_label: String,
    rights: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    help: &'static str,
    /// A token for it is in the vault (its value never leaves the core).
    has_token: bool,
}

fn catalog() -> Vec<ServiceInfo> {
    services::REGISTRY
        .iter()
        .map(|service| {
            let spec = service.spec();
            let (custom_url, url_required) = match spec.address {
                Address::Fixed(_) => (None, false),
                Address::Custom { default, required } => (Some(default), required),
            };
            ServiceInfo {
                id: spec.id,
                name: spec.name,
                palette: spec.palette,
                kind: spec.kind,
                custom_url,
                url_required,
                account: spec.account.map(|a| AccountInfo { label: tr(a.label), hint: tr(a.hint) }),
                refresh_minutes: spec.refresh.as_secs().div_ceil(60),
                token_label: tr(spec.token_label),
                rights: tr(spec.rights),
                warning: spec.warning.map(tr),
                help: spec.help,
                has_token: crate::vault::has(&format!("service:{}", spec.id)),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------------------
// Commands

#[tauri::command]
pub fn pastilles_snapshot(hub: State<'_, PastilleHub>) -> Vec<PastilleView> {
    hub.snapshot()
}

/// The services îlot knows, and which have a token stored.
#[tauri::command]
pub async fn pastilles_catalog() -> Result<Vec<ServiceInfo>, String> {
    tauri::async_runtime::spawn_blocking(catalog).await.map_err(|e| e.to_string())
}

/// « Réessayer » / a token just stored: read this pastille again soon (30 s apart at most).
#[tauri::command]
pub fn pastille_refresh(id: String, hub: State<'_, PastilleHub>) {
    let inner = hub.inner.lock().unwrap();
    if let Some(running) = inner.running.iter().find(|r| r.job.id() == id) {
        running.wake.notify_one();
    }
}

/// Brings the main tool chosen in the settings to the front. The interface sends nothing:
/// the tool comes from the settings, the window from Windows.
#[tauri::command]
pub async fn pastille_show_tool(app: AppHandle) -> Result<(), String> {
    let id = app.state::<crate::settings::SettingsStore>().get().pastilles.tool;
    let tool = tools::find(&id).ok_or(Text::NoMainTool)?;
    tauri::async_runtime::spawn_blocking(move || tools::bring_to_front(tool)).await.map_err(|e| e.to_string())?
}

async fn pull(app: AppHandle, model: String) {
    let hub = app.state::<PastilleHub>();
    let finish = |pull: Pull| {
        if let Some((current, _)) = hub.inner.lock().unwrap().pull.as_mut() {
            *current = pull;
        }
        emit(&app);
    };
    let mut progress = Pull::new(&model);
    let request = hub.http().map(|http| {
        http.client()
            .post(format!("{}/api/pull", local::OLLAMA))
            .timeout(PULL_TIMEOUT)
            .json(&serde_json::json!({ "model": model, "stream": true }))
    });
    let sent = match request {
        Ok(request) => request.send().await.ok(),
        Err(_) => None,
    };
    let Some(mut response) = sent.filter(|response| response.status().is_success()) else {
        progress.error = Some(tr(Text::OllamaPullRefused));
        progress.done = true;
        return finish(progress);
    };
    let mut buffer = Vec::new();
    let mut told = Instant::now();
    while let Ok(Some(chunk)) = response.chunk().await {
        buffer.extend_from_slice(&chunk);
        while let Some(end) = buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = buffer.drain(..=end).collect();
            progress.read_line(&String::from_utf8_lossy(&line));
        }
        if buffer.len() > http::MAX_BODY {
            break;
        }
        if told.elapsed() >= PULL_EVERY {
            told = Instant::now();
            finish(progress.clone());
        }
        if progress.done {
            break;
        }
    }
    if !progress.done {
        progress.error = Some(tr(Text::PullStopped));
        progress.done = true;
    }
    finish(progress);
    // The new model shows in the list at once.
    if let Some(running) = hub.inner.lock().unwrap().running.iter().find(|r| r.job == Job::Local(Server::Ollama)) {
        running.wake.notify_one();
    }
}

/// « Télécharger un modèle »: starts an Ollama download the user asked for by name. One at
/// a time; only while the Ollama pastille is on.
#[tauri::command]
pub fn ollama_pull(model: String, app: AppHandle) -> Result<(), String> {
    let model = model.trim().to_owned();
    if !local::valid_model(&model) {
        return Err(Text::ModelNameInvalid.into());
    }
    let hub = app.state::<PastilleHub>();
    {
        let mut inner = hub.inner.lock().unwrap();
        if !inner.running.iter().any(|r| r.job == Job::Local(Server::Ollama)) {
            return Err(Text::AddOllamaFirst.into());
        }
        if inner.pull.as_ref().is_some_and(|(pull, _)| !pull.done) {
            return Err(Text::PullRunning.into());
        }
        let task = tauri::async_runtime::spawn(pull(app.clone(), model.clone()));
        inner.pull = Some((Pull::new(&model), task));
    }
    emit(&app);
    Ok(())
}

/// Stops the download (Ollama keeps what it already has) or clears a finished one.
#[tauri::command]
pub fn ollama_pull_cancel(app: AppHandle) {
    let hub = app.state::<PastilleHub>();
    if let Some((_, task)) = hub.inner.lock().unwrap().pull.take() {
        task.abort();
    }
    emit(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: Duration = Duration::from_secs(60);

    #[test]
    fn a_service_keeps_its_pace_and_backs_off_within_bounds() {
        let refresh = 3 * MIN;
        assert_eq!(next_wait(refresh, &Ok(()), 0), refresh);
        assert_eq!(next_wait(refresh, &Err(Failure::Unreachable), 1), 6 * MIN);
        assert_eq!(next_wait(refresh, &Err(Failure::Unreachable), 2), 12 * MIN);
        assert_eq!(next_wait(refresh, &Err(Failure::Server(502)), 9), SERVICE_MAX);
        assert_eq!(next_wait(refresh, &Err(Failure::Refused), 1), SERVICE_MAX);
        assert_eq!(next_wait(refresh, &Err(Failure::NoToken), 1), SERVICE_MAX);
        // A limited service gets what it asks for, never less than its pace, at most an hour.
        assert_eq!(next_wait(refresh, &Err(Failure::Limited(Some(30))), 1), refresh);
        assert_eq!(next_wait(refresh, &Err(Failure::Limited(Some(900))), 1), 15 * MIN);
        assert_eq!(next_wait(refresh, &Err(Failure::Limited(Some(99_999))), 1), LIMIT_MAX);
        assert_eq!(next_wait(refresh, &Err(Failure::Limited(None)), 1), 6 * MIN);
    }

    #[test]
    fn every_wait_is_bounded_for_every_service() {
        let failures = [
            Failure::Unreachable,
            Failure::Refused,
            Failure::Limited(None),
            Failure::Limited(Some(u64::MAX / 4)),
            Failure::Server(500),
            Failure::BadAnswer,
            Failure::NoToken,
            Failure::Setup(Text::AccountIdMissing),
        ];
        for service in services::REGISTRY {
            let refresh = service.spec().refresh;
            for failure in &failures {
                for n in [0, 1, 5, 40, u32::MAX] {
                    let wait = next_wait(refresh, &Err(failure.clone()), n);
                    assert!(wait >= refresh && wait <= LIMIT_MAX, "{} {failure:?} {n}: {wait:?}", service.spec().id);
                }
            }
        }
    }

    #[test]
    fn a_local_server_is_asked_slowly_once_it_stopped() {
        assert_eq!(local_wait(true, 0), Duration::from_secs(15));
        assert_eq!(local_wait(false, 1), Duration::from_secs(30));
        assert_eq!(local_wait(false, 2), Duration::from_secs(60));
        assert_eq!(local_wait(false, 50), Duration::from_secs(60));
    }

    #[test]
    fn views_never_carry_a_token() {
        let view = PastilleView { line: "3 PR ouvertes".into(), ..PastilleView::waiting("github") };
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.to_lowercase().contains("token") && !json.contains("jeton-de-test"));
    }

    #[test]
    fn the_local_offset_is_a_whole_number_of_minutes() {
        let offset = local_offset_min(now_ms());
        assert!((-14 * 60..=14 * 60).contains(&offset));
    }

    #[test]
    fn the_catalog_and_a_new_pastille_speak_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(PastilleView::waiting("github").line, "Reading…");
        let github = catalog().into_iter().find(|s| s.id == "github").unwrap();
        assert_eq!(github.token_label, "Fine-grained access token");
        let cloudflare = catalog().into_iter().find(|s| s.id == "cloudflare").unwrap();
        assert_eq!(cloudflare.account.unwrap().label, "Account ID");
    }
}
