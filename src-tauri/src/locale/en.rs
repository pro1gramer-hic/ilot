//! English: short and natural, in the same warm tone as the French, with English
//! punctuation (no space before « : » or « ? »).

use super::Text as T;
use super::{Lang, Text};

/// Every text of the core in this language.
pub fn text(text: Text<'_>) -> String {
    match text {
        // The notification area (tray.rs)
        T::TraySettings => "Settings".into(),
        T::TrayDoNotDisturb => "Do not disturb".into(),
        T::TrayQuit => "Quit".into(),

        // Commands of main.rs
        T::StatsClearFailed { error } => format!("Couldn't clear: {error}"),
        T::NoSessionToJoin => "No session to go to.".into(),
        T::RelayMissing { path } => format!("Relay not found: {path}"),
        T::InstallationMissing { id } => format!("Installation not found: {id}"),
        T::ConnectorUnknown => "Unknown connector".into(),
        T::ConnectFailed { error } => format!("Failed: {error}"),
        T::AutostartFailed { error } => format!("Start with Windows: {error}"),

        // What an agent does (connectors/activity.rs): the verb before its target
        T::VerbRun => "runs".into(),
        T::VerbRead => "reads".into(),
        T::VerbEdit => "edits".into(),
        T::VerbWrite => "writes".into(),
        T::VerbBrowse => "lists".into(),
        T::VerbSearch => "searches".into(),
        T::VerbFetch => "browses".into(),
        T::VerbDelegate => "delegates".into(),
        T::VerbPlan => "plans".into(),
        T::VerbAsk => "asks".into(),
        T::VerbUse => "uses".into(),
        T::VerbThink => "thinks".into(),
        T::VerbWork => "works".into(),
        T::ToolUnnamed => "tool".into(),

        // What an agent's session does (connectors)
        T::AgentStarts => "starts".into(),
        T::AgentClosedSession => "closed the session".into(),
        T::AgentFinished => "finished".into(),
        T::AgentFailed => "failed".into(),
        T::AgentInterrupted => "interrupted".into(),
        T::AgentWaitsForOk => "waits for your OK".into(),
        T::AgentWaitsForOkInTerminal => "waits for your OK in its terminal".into(),
        T::AgentAsks => "has a question for you".into(),
        T::AgentAsksInTerminal => "has a question for you in its terminal".into(),
        T::AgentTakesOver => "takes over again".into(),
        T::AgentNotification => "notification".into(),
        T::OtherAgent { name } => format!("{name} (other)"),
        T::ConnectorOther => "Other".into(),

        // Permissions and questions (connectors/hook_events.rs, claude_code.rs)
        T::WantsToRun { agent } => format!("{agent} wants to run a command"),
        T::WantsToEditEverywhere { agent } => format!("{agent} wants to change a file everywhere the text appears"),
        T::WantsToEditFile { agent } => format!("{agent} wants to change a file"),
        T::WantsToEditFiles { agent } => format!("{agent} wants to change files"),
        T::WantsToFetch { agent } => format!("{agent} wants to open a page"),
        T::WantsToUse { agent, tool } => format!("{agent} wants to use {tool}"),
        T::ClaudeAsks { count } => if count == 1 { "Claude has a question for you".into() } else { format!("Claude has {count} questions for you") },

        // Plugging an AI in (connectors, wsl.rs)
        T::AmpInWsl => "Amp runs in WSL: connect it in your distribution.".into(),
        T::KiroInWsl => "Kiro connects in WSL only for now.".into(),
        T::NothingToPlugOnWindows => "Nothing to connect on Windows: ilot-cli.exe comes with îlot.".into(),
        T::NotWrittenByIlot { path } => format!("{path} already exists and was not written by îlot: rename it first."),
        T::CommandMissing { path } => format!("Command not found: {path}"),
        T::UnsafeArgument { arg } => format!("unsafe argument: {arg}"),
        T::PathNotForCmd { path } => format!("path cmd cannot take: {path}"),
        T::FileChanging { path } => format!("{path} is changing right now: try again in a moment."),
        T::NotAWslFile { path } => format!("not a WSL file: {path}"),
        T::UnreadableLink { path } => format!("unreadable link: {path}"),
        T::UnreadableMode { path } => format!("unreadable permissions: {path}"),

        // What makes a shell command risky (risk.rs)
        T::RiskDelete => "Deletes files for good.".into(),
        T::RiskGitClean => "Deletes the files git does not track, with no way back.".into(),
        T::RiskGitResetHard => "Erases uncommitted changes, with no way back.".into(),
        T::RiskWslUnregister => "Deletes a whole WSL distribution.".into(),
        T::RiskPushForce => "Pushes to GitHub with force: the remote history will be overwritten.".into(),
        T::RiskPush => "Publishes your commits to the remote repository.".into(),
        T::RiskPushDelete => "Deletes a branch of the remote repository.".into(),
        T::RiskNetwork => "Uses the network.".into(),
        T::RiskRemoteShell => "Opens a session on another machine.".into(),
        T::RiskRemoteCopy => "Copies files to or from another machine.".into(),
        T::RiskRawSocket => "Opens a direct network connection.".into(),
        T::RiskPipeToShell => "Runs a downloaded script without showing it to you.".into(),
        T::RiskEncoded => "Runs an encoded command that can't be read here.".into(),
        T::RiskAdmin => "Asks for administrator rights.".into(),
        T::RiskWorldWritable => "Makes files writable by everyone.".into(),
        T::RiskSystemFiles => "Touches system files.".into(),
        T::RiskRegistry => "Changes the Windows registry.".into(),
        T::RiskServices => "Stops, starts or changes system services.".into(),
        T::RiskPower => "Shuts down or restarts the computer.".into(),
        T::RiskDisk => "Can wipe a whole disk.".into(),
        T::RiskSecurity => "Changes a system protection (firewall, antivirus, execution policy).".into(),
        T::RiskAccounts => "Changes the computer's accounts.".into(),
        T::RiskTooNested => "Too nested to be checked in full.".into(),

        // « OK Mochi », Mochi's voice and the dictation (voice/, dictation.rs)
        T::VoicePhrasesRefused => "Phrase list refused.".into(),
        T::VoiceFailedToStart { error } => format!("Speech recognition didn't start: {error}"),
        T::VoiceNoRecognizer { lang } => match lang {
            Lang::Fr => "No French speech recognizer is installed on this PC.".into(),
            Lang::En => "No English speech recognizer is installed on this PC.".into(),
        },
        T::VoiceNoMicrophone => "Can't reach the microphone. Check Windows Settings › Privacy & security › Microphone.".into(),
        T::VoiceNotStarted => "Speech recognition didn't start.".into(),
        T::VoiceNotAnswering => "Speech recognition isn't responding.".into(),
        T::VoiceSilent => "Mochi's voice isn't responding.".into(),
        T::NoVoice { lang } => match lang {
            Lang::Fr => "No French voice is installed on this PC.".into(),
            Lang::En => "No English voice is installed on this PC.".into(),
        },
        T::DictationUnavailable { lang } => match lang {
            Lang::Fr => "Dictation unavailable: Windows' French speech recognition is not installed.".into(),
            Lang::En => "Dictation unavailable: Windows' English speech recognition is not installed.".into(),
        },
        T::DictationNoMicrophone => "Can't reach the microphone. Allow desktop apps in Settings › Privacy & security › Microphone.".into(),
        T::AskNoKey => "I don't have your Claude key. Add it in the chat settings, then ask me again.".into(),
        T::AskRefused => "Claude didn't want to answer that.".into(),
        T::AskKeyRejected => "Claude turned down your key. Check it in the chat settings.".into(),
        T::AskBusy => "Claude is very busy. Try again in a moment.".into(),
        T::AskUnreachable => "I can't reach Claude. Check your connection.".into(),
        T::AskUnexpected { status } => format!("Claude answered with an error ({status})."),
        T::AskSystem => "You are Mochi, the little mascot of îlot, an island at the top of the screen that follows the \
            user's coding agents. You were spoken to out loud and your answer will be read by a synthetic \
            voice: answer in English, in two or three short, natural sentences, with no list, no heading and \
            no formatting. Rely only on the data given; if a piece of information is missing, just say so.".into(),
        T::AskContent { question, context } => format!("{question}\n\nîlot data:\n{context}"),

        // The chat (chat/), its attachments (attachments.rs) and the window Mochi reads (window_context.rs)
        T::ChatEmptyQuestion => "Write a question".into(),
        T::ChatBadRequest => "Invalid request".into(),
        T::ChatMessageTooLong { max } => format!("Message too long: {max} characters at most."),
        T::ChatContextAttached { title, app, text } => format!("Context attached by the user: the window “{title}” ({app}).\n\"\"\"\n{text}\n\"\"\""),
        T::ChatFileAttached { name, text } => format!("Attached file: {name}\n\"\"\"\n{text}\n\"\"\""),
        T::ChatImageAttached { name } => format!("Attached image: {name}"),
        T::ChatWhatDoYouThink => "What do you think?".into(),
        T::ChatNoClient => "Network client unavailable".into(),
        T::ChatKeyInvalid => "Invalid key: save it again".into(),
        T::ChatStoppedAnswering { provider } => format!("{provider} stopped answering."),
        T::ChatProviderError { provider, message } => format!("{provider}: {message}"),
        T::ChatMissingKey { provider } => format!("Add your {provider} key in Settings › Chat."),
        T::ChatMalformed => "Unreadable answer from the provider".into(),
        T::ChatUnknownError => "unknown error".into(),
        T::GeminiBlockedAnswer => "Gemini blocked the answer (Google's safety filter)".into(),
        T::GeminiBlockedQuestion => "Gemini blocked the question (Google's safety filter)".into(),
        T::ProviderKeyRefused { name } => format!("{name} refused the key. Check it in Settings › Chat."),
        T::ProviderModelMissing { name } => format!("{name} doesn't have this model. Pick another one."),
        T::ProviderTooLong { name } => format!("Message too long for {name}."),
        T::ProviderLimited { name } => format!("{name} is limiting your requests (quota or credit). Try again later."),
        T::ProviderDown { name } => format!("{name} is overloaded or down. Try again in a moment."),
        T::ProviderRefusedBecause { name, status, detail } => format!("{name} refused the request ({status}): {detail}"),
        T::ProviderRefused { name, status } => format!("{name} refused the request ({status})."),
        T::OllamaDown => "Ollama isn't answering on this PC. Start Ollama, then try again.".into(),
        T::LmStudioDown => "LM Studio isn't answering. Start its local server (port 1234), then try again.".into(),
        T::ProviderUnreachable { name } => format!("Couldn't reach {name}. Check your Internet connection."),
        T::AttachmentNotDropped => "This file wasn't dropped on the island".into(),
        T::AttachmentUnknown => "Attachment not found: drop the file again".into(),
        T::FileCannotOpen { name } => format!("Couldn't open {name}"),
        T::FileCannotRead { name } => format!("Couldn't read {name}"),
        T::FileTooBig { name, kilobytes } => format!("{name} is too big for the chat ({kilobytes} KB at most)."),
        T::FileNotTextOrImage { name } => format!("Mochi can only read text and images (PNG, JPEG, GIF, WebP): {name} is neither."),
        T::ContextUnknown => "This window is no longer offered: drop Mochi on it again".into(),
        T::ContextNotConfirmed => "This window's text wasn't attached to the chat".into(),
        T::ContextNothing => "Mochi found no readable text in this window".into(),
        T::ContextGone => "This window is closed or not responding".into(),
        T::BlockPasswordManager => "Password manager: îlot doesn't read this window.".into(),
        T::BlockBank => "Bank or payment: îlot doesn't read this window.".into(),
        T::BlockSignIn => "Sign-in page or security code: îlot doesn't read this window.".into(),
        T::BlockSystem => "Windows security window: îlot doesn't read it.".into(),
        T::BlockPasswordField => "This window has a password field: îlot doesn't read it.".into(),
        T::BlockAdmin => "Program run as administrator: Windows keeps îlot from reading this window.".into(),

        // The keys of the vault (vault.rs, vault/credential_manager.rs)
        T::SecretBadName => "Secret name refused".into(),
        T::KeyEmpty => "The key is empty".into(),
        T::KeyTooLong => "Key too long: 2,560 bytes at most".into(),
        T::KeyControl => "The key contains invisible characters".into(),
        T::KeyUnreadable => "Unreadable key: save it again".into(),

        // E-mail (mail.rs)
        T::MailBadAddress { address } => format!("Invalid address: {address}"),
        T::MailTooManyRecipients { max } => format!("{max} recipients at most"),
        T::MailSubjectTooLong { max } => format!("Subject too long: {max} characters at most"),
        T::MailTooLong => "Message too long".into(),
        T::MailInvalid => "Invalid message".into(),
        T::MailYourApp => "your mail app".into(),
        T::MailYourMail => "your mail app".into(),
        T::MailCannotOpen => "Couldn't open the mail app".into(),
        T::MailClosedUnsent { client } => format!("Message closed in {client} without sending."),
        T::MailCopiedForOutlook => "Subject and text copied: paste them into Outlook.".into(),
        T::MailNoApp => "No mail app answered.".into(),
        T::MailDragFile { name } => format!("Drag {name} into the message: its folder is open."),
        T::MailTextCopied => "Text too long for the link: it's copied in full.".into(),

        // Music, videos and the windows they play in (media.rs, video.rs, players.rs, on_top.rs, source_app.rs, app_volume.rs, native_host.rs)
        T::MediaNoAccess => "Windows refuses access to the media playing. Restart îlot to try again.".into(),
        T::MediaNothingPlaying => "Nothing is playing.".into(),
        T::MediaRefused => "The app didn't accept the command.".into(),
        T::MediaPlayerApp => "Media Player".into(),
        T::OtherApp => "Other app".into(),
        T::VideoInvalidLink => "This link doesn't lead to a YouTube video.".into(),
        T::VideoNoWindow => "No video is open.".into(),
        T::VideoNotABar => "This window controls no video.".into(),
        T::VideoBarTitle => "Video controls · îlot".into(),
        T::MiniPlayerTitle => "Mini player · îlot".into(),
        T::VideoWindowMissing => "Couldn't find this video's window.".into(),
        T::WindowMoveRefused => "Windows didn't let îlot move this window.".into(),
        T::AppWindowMissing => "Couldn't find this app's window.".into(),
        T::SpotifySilent => "Spotify isn't playing any sound right now.".into(),
        T::VolumeSpotifyOnly => "Volume can be set for Spotify only.".into(),
        T::ExtensionNotConnected => "îlot's browser extension isn't connected.".into(),
        T::BrowserNoAnswer => "The browser didn't answer.".into(),

        // Messages of the other apps (notifications.rs)
        T::WebsitesApp => "Websites".into(),
        T::MessageGone => "This message is no longer available.".into(),
        T::BrowserNotOpen => "The browser isn't open.".into(),
        T::AppNotOpen { app } => format!("{app} isn't open."),
        T::WindowsSettingsMissing => "Windows Settings not found.".into(),

        // The island, Mochi on the desktop, the screens (window.rs, buddy.rs, screens.rs)
        T::MoveInvalid => "Invalid move".into(),
        T::IslandWindowMissing => "Island window not found".into(),
        T::ScreenMissing => "Screen not found".into(),
        T::ScreenNumber { n } => format!("Screen {n}"),
        T::PositionInvalid => "Invalid position".into(),
        T::ModeUnknown { mode } => format!("Unknown mode: {mode}"),
        T::MochiWindowMissing => "Mochi's window not found".into(),
        T::IslandNotPlaced => "The island isn't placed yet".into(),

        // Shortcuts, sounds, clipboard, report (shortcuts.rs, sounds.rs, clipboard.rs, export.rs)
        T::ShortcutInvalid { shortcut } => format!("Invalid shortcut: {shortcut}"),
        T::ShortcutWindowsKey => "The Windows key is reserved for Windows.".into(),
        T::ShortcutF12 => "Windows reserves F12 for debugging.".into(),
        T::ShortcutNeedsModifier => "Add Ctrl or Alt: on its own, this key would stop working elsewhere.".into(),
        T::ActionUnknown { action } => format!("Unknown action: {action}"),
        T::IslandKeepsShortcut => "The island always keeps a shortcut: it's how it opens from the keyboard.".into(),
        T::ShortcutUsedByIlot => "This shortcut is already used by another îlot action.".into(),
        T::ShortcutTaken { shortcut } => format!("{shortcut} is already taken by Windows or another app. The old shortcut stays."),
        T::SoundUnknown => "Unknown sound".into(),
        T::SoundNoFile => "No file for this sound".into(),
        T::SoundTooHeavy => "File too large (2 MB at most)".into(),
        T::SoundUnreadable => "Unreadable sound".into(),
        T::SoundBusy => "Sound busy".into(),
        T::SoundsFolderMissing => "Sounds folder not found".into(),
        T::SoundsFolderFailed { error } => format!("Sounds folder: {error}"),
        T::SoundsFolderCannotOpen => "Couldn't open the sounds folder".into(),
        T::ClipboardTooLong => "Text too long for the clipboard.".into(),
        T::ClipboardInvalid => "This text can't go in the clipboard.".into(),
        T::ClipboardBusy => "Another app is using the clipboard. Try again.".into(),
        T::ClipboardFailed => "Windows didn't accept the copy.".into(),
        T::ReportFileName => "îlot report".into(),
        T::ReportSaveTitle => "Save the report".into(),
        T::ReportImageFilter => "PNG image".into(),
        T::ReportDialogClosed => "The save window closed.".into(),
        T::ReportImageMissing => "Image missing.".into(),
        T::ReportImageRefused => "Image refused: it isn't the report.".into(),
        T::SaveFailed { error } => format!("Couldn't save: {error}"),

        // Pastilles: the main tool, the local servers, the services (pastilles/)
        T::PastilleReading => "Reading…".into(),
        T::ToolClosed { tool } => format!("{tool} isn't open"),
        T::ToolOpenOn { tool, project } => format!("{tool} open on {project}"),
        T::ToolOpen { tool } => format!("{tool} is open"),
        T::ToolNotOpen { tool } => format!("{tool} isn't open."),
        T::ToolFrontRefused { tool } => format!("Windows didn't let îlot show {tool}."),
        T::NoMainTool => "No main tool chosen.".into(),
        T::OllamaPullRefused => "Ollama didn't accept the download.".into(),
        T::PullStopped => "The download stopped.".into(),
        T::ModelNameInvalid => "Invalid model name (for example: gemma4, llama3.2:3b).".into(),
        T::AddOllamaFirst => "Add the Ollama widget first.".into(),
        T::PullRunning => "A download is already running.".into(),
        T::PullPreparing => "Preparing".into(),
        T::PullDone => "Done".into(),
        T::PullVerifying => "Verifying".into(),
        T::PullDownloading => "Downloading".into(),
        T::PullFinishing => "Finishing".into(),
        T::LocalNotRunning { name } => format!("{name} isn't running"),
        T::LocalNeedsToken { name } => format!("{name} asks for a token: îlot only reads without authentication"),
        T::LocalOdd { name } => format!("{name} is answering in an unexpected way"),
        T::NoModelInstalled => "No model installed".into(),
        T::NoModelLoaded { n } => format!("No model in memory, {n} installed"),
        T::ModelLoaded { name } => format!("{name} in memory"),
        T::ModelsLoaded { n } => format!("{n} models in memory"),
        T::ServiceUnreachable => "Unreachable for now".into(),
        T::ServiceTokenRefused => "Token refused: check its rights".into(),
        T::ServiceLimited => "Request limit reached, îlot is waiting".into(),
        T::ServiceError { code } => format!("The service is answering badly (error {code})"),
        T::ServiceBadAnswer => "Unexpected answer from the service".into(),
        T::ServiceNoToken => "No token: add it in Settings".into(),
        T::AccountIdMissing => "Account ID missing".into(),
        T::UsernameMissing => "Username missing".into(),

        // What the services say (pastilles/services/): states, counts, cards
        T::StateLive => "live".into(),
        T::StateFailed => "failed".into(),
        T::StateDeploying => "deploying".into(),
        T::StateCanceled => "canceled".into(),
        T::StateWaiting => "waiting".into(),
        T::StateIdle => "idle".into(),
        T::StateBlocked => "blocked".into(),
        T::StateDeleted => "deleted".into(),
        T::StateBuilding => "building".into(),
        T::StateUnknown => "unknown state".into(),
        T::Deploying => "Deploying…".into(),
        T::CountFailed { n } => format!("{n} failed"),
        T::CountDeploying { n } => format!("{n} deploying"),
        T::CountLive { n } => format!("{n} live"),
        T::ToReview { n } => format!("{n} to review"),
        T::ReviewsRequested { n } => counted(n, "review requested", "reviews requested"),
        T::MetricToday => "Today".into(),
        T::MetricUpcoming => "Upcoming".into(),
        T::MetricLive => "Live".into(),
        T::MetricRunning => "Running".into(),
        T::MetricFailed => "Failed".into(),
        T::MetricToReview => "To review".into(),

        // Cal.com
        T::CalcomTokenLabel => "API key (cal_live_…)".into(),
        T::CalcomRights => "Cal.com has no read-only key: îlot only reads your upcoming bookings.".into(),
        T::CalcomWarning => "A Cal.com key has many rights. îlot only reads your next bookings: give it an expiry date.".into(),
        T::CalcomToday { clock } => format!("today at {clock}"),
        T::CalcomTomorrow { clock } => format!("tomorrow at {clock}"),
        T::CalcomOn { day, month, clock } => format!("on {} {} at {clock}", month_name(month), day.trim_start_matches('0')),
        T::CalcomToConfirm => "To confirm".into(),
        T::CalcomBooking => "Booking".into(),
        T::CalcomNone => "No upcoming bookings".into(),
        T::CalcomFree => "Calendar free".into(),
        T::CalcomNext { title, when } => format!("Next: {title}, {when}"),
        T::CalcomShort { clock } => format!("Next {clock}"),

        // Cloudflare
        T::CloudflareAccountLabel => "Account ID".into(),
        T::CloudflareAccountHint => "32 characters, shown in the dashboard, on the right of the account's home page.".into(),
        T::CloudflareTokenLabel => "Custom API token".into(),
        T::CloudflareRights => "Read-only, limited to this account: Cloudflare Pages (Read), Workers Scripts (Read), \
            Notifications (Read).".into(),
        T::CloudflarePages { n } => counted(n, "Pages project", "Pages projects"),
        T::CloudflareWorkers { n } => counted(n, "Worker", "Workers"),
        T::CloudflarePagesFailed => "Failed Pages".into(),
        T::CloudflareWorkersMetric => "Workers".into(),
        T::CloudflareAlerts { n } => counted(n, "alert", "alerts"),
        T::CloudflareAlertsMetric => "Alerts 24 h".into(),

        // Docker Hub
        T::DockerUserLabel => "Docker Hub username".into(),
        T::DockerUserHint => "The token's own: îlot lists this account's images.".into(),
        T::DockerTokenLabel => "Personal access token (dckr_pat_…)".into(),
        T::DockerRights => "Repo Read-only (or Public Repo Read-only if everything is public).".into(),
        T::DockerTagPushed => "Tag pushed".into(),
        T::DockerPrivate => "Private image".into(),
        T::DockerPublic => "Public image".into(),
        T::DockerNone => "No images".into(),
        T::DockerLatest { n, latest } => format!("{}, last updated: {latest}", counted(n, "image", "images")),
        T::DockerImages => "Images".into(),
        T::DockerPulls => "Pulls".into(),
        T::DockerRecentTags => "Recent tags".into(),

        // Dokploy
        T::DokployTokenLabel => "API key (Profile, API/CLI section)".into(),
        T::DokployRights => "Dokploy has no read-only key: îlot only reads.".into(),
        T::DokployWarning => "The Dokploy key gives every right on the account. îlot only reads, but keep it for îlot alone \
            and give it an expiry date.".into(),
        T::DokployNone => "No application found".into(),
        T::DokployApps { n } => counted(n, "application", "applications"),
        T::DokployUnnamed => "Unnamed".into(),

        // GitHub, GitLab
        T::GithubTokenLabel => "Fine-grained access token".into(),
        T::GithubRights => "Read-only: Pull requests and Commit statuses with read access, on the chosen repositories.".into(),
        T::GithubDraft { repo } => format!("{repo}, draft"),
        T::GithubNoPr => "No open PRs".into(),
        T::GithubOpenPrs { n } => counted(n, "open PR", "open PRs"),
        T::GithubCiFailing { n } => format!("CI failing on {n}"),
        T::GithubShort { n } => counted(n, "PR", "PRs"),
        T::GithubOpenMetric => "Open PRs".into(),
        T::GithubCiMetric => "CI failing".into(),
        T::GitlabTokenLabel => "Personal access token".into(),
        T::GitlabRights => "read_api scope only (reads the API).".into(),
        T::GitlabNoMr => "No open MRs".into(),
        T::GitlabOpenMrs { n } => counted(n, "open MR", "open MRs"),
        T::GitlabPipelineFailed => "last pipeline failed".into(),
        T::GitlabPipelinesRunning { n } => counted(n, "pipeline running", "pipelines running"),
        T::GitlabShort { n } => counted(n, "MR", "MRs"),
        T::GitlabOpenMetric => "Open MRs".into(),
        T::GitlabPipelinesMetric => "Failed pipelines".into(),

        // n8n, Notion, Resend, Stripe, Vercel
        T::N8nTokenLabel => "API key (Settings, n8n API)".into(),
        T::N8nRights => "On Enterprise: execution:read scopes only. Otherwise the key has every right: give it an expiry \
            date.".into(),
        T::N8nWarning => "Outside Enterprise, an n8n key gives every right on the account. îlot only reads the executions.".into(),
        T::N8nNoneToday => "No executions today".into(),
        T::N8nToday { n } => format!("{} today", counted(n, "execution", "executions")),
        T::N8nWorkflow { id } => format!("Workflow {id}"),
        T::N8nFailedExecution => "Failed execution".into(),
        T::N8nShort { n } => format!("{n} runs"),
        T::NotionTokenLabel => "Internal integration secret".into(),
        T::NotionRights => "“Read content” capability only, with no user information. Then share the pages you want with the \
            connection.".into(),
        T::NotionUntitled => "Untitled".into(),
        T::NotionNone => "No page shared with îlot".into(),
        T::NotionLatest { title } => format!("Last edited: {title}"),
        T::NotionEditedToday => "Edited today".into(),
        T::NotionRecent => "Recent".into(),
        T::NotionPageEdited => "Page edited".into(),
        T::NotionToday { n, line } => format!("{} today. {line}", counted(n, "page edited", "pages edited")),
        T::ResendTokenLabel => "API key (re_…)".into(),
        T::ResendRights => "Resend has no read-only key: listing e-mails probably needs a “Full access” key.".into(),
        T::ResendWarning => "Careful: Resend has no read-only key. A “Full access” key can also send and delete. îlot only \
            reads, but create a key for îlot alone.".into(),
        T::ResendNoneToday => "No e-mails sent today".into(),
        T::ResendToday { n } => format!("{} today", counted(n, "e-mail sent", "e-mails sent")),
        T::ResendShortSent { n } => format!("{n} sent"),
        T::ResendBounces { n } => counted(n, "bounce", "bounces"),
        T::ResendBounce => "Bounce".into(),
        T::ResendSent => "Sent".into(),
        T::ResendBouncesMetric => "Bounces".into(),
        T::ResendComplaints => "Complaints".into(),
        T::StripeTokenLabel => "Restricted key (rk_live_…)".into(),
        T::StripeRights => "Restricted key: PaymentIntents with read access (Read), everything else on None.".into(),
        T::StripeNoneToday => "No payments today".into(),
        T::StripeToday { n, amount } => format!("{} today, {amount}", counted(n, "payment", "payments")),
        T::Money { sign, whole, cents, symbol } => {
            let number = format!("{}{}", grouped(whole, ','), if cents.is_empty() { String::new() } else { format!(".{cents}") });
            if symbol.chars().count() == 1 { format!("{sign}{symbol}{number}") } else { format!("{sign}{number} {symbol}") }
        },
        T::StripeOtherCurrencies => "(and other currencies)".into(),
        T::StripePayment => "Payment".into(),
        T::StripePaymentAt { clock } => format!("Payment at {clock}"),
        T::StripePayments => "Payments".into(),
        T::StripeAmount => "Today's amount".into(),
        T::StripeDeclined => "Declined".into(),
        T::VercelTokenLabel => "Access token".into(),
        T::VercelRights => "The narrowest scope possible: a single project, or a team.".into(),
        T::VercelWarning => "Vercel has no read-only token: îlot only reads, but limit its scope to one project.".into(),
        T::VercelNone => "No deployments".into(),
        T::VercelLatest { state, name } => format!("Last deployment {state}: {name}"),
        T::VercelProduction => "Production".into(),
        T::VercelPreview => "Preview".into(),
        T::VercelLatestMetric => "Latest".into(),

        // « Aller au terminal » (terminal.rs)
        T::TerminalNotFound => "No terminal window found for this session.".into(),
        T::TerminalNotInFront => "Windows didn't let îlot bring the window to the front.".into(),
        T::TerminalTabInFront { project } => format!("Windows Terminal brought to the front, on the tab of project {project}."),
        T::TerminalInFront => "Terminal brought to the front, tab not identified.".into(),
        T::EditorOpenOn { editor, project } => format!("{editor} open on {project}."),
        T::EditorInFront { editor } => format!("{editor} brought to the front, project window not identified."),
        T::ConsoleOfProjectInFront { project } => format!("Terminal of project {project} brought to the front."),
        T::ConsoleInFront => "Terminal brought to the front, window not identified.".into(),

        // The Windows Credential Manager (vault/credential_manager.rs)
        T::CredentialComment => "Key kept by îlot".into(),
        T::CredentialSaveFailed { code } => format!("Windows Credential Manager: couldn't save ({code})"),
        T::CredentialReadFailed { code } => format!("Windows Credential Manager: couldn't read ({code})"),
        T::CredentialRemoveFailed { code } => format!("Windows Credential Manager: couldn't delete ({code})"),
        T::CredentialListFailed { code } => format!("Windows Credential Manager: couldn't list ({code})"),
    }
}

/// « 1 open PR », « 0 open PRs », « 3 open PRs »: singular for one only.
fn counted(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// « 1,234,567 »: thousands apart by `separator`.
fn grouped(n: u64, separator: char) -> String {
    let digits = n.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(separator);
        }
        grouped.push(c);
    }
    grouped
}

/// The short English name of a month from its two digits (« 03 » → « Mar »).
fn month_name(digits: &str) -> &str {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    digits.parse::<usize>().ok().and_then(|m| MONTHS.get(m.wrapping_sub(1))).copied().unwrap_or(digits)
}
