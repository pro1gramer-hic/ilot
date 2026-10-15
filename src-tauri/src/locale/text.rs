//! Every text the core shows the user, by name. Each language says them all in its own
//! file (fr.rs, en.rs): one `match` over this list, which the compiler wants complete. A
//! text that takes a number or a name carries it, so that each language places it and
//! makes its own plurals.

use super::Lang;

/// Declares `Text` and, for the tests, one sample of each text.
macro_rules! texts {
    ($($(#[$doc:meta])* $name:ident $({ $($field:ident: $kind:ty),* $(,)? })?),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Text<'a> {
            $($(#[$doc])* $name $({ $($field: $kind),* })?),*
        }

        #[cfg(test)]
        impl<'a> Text<'a> {
            /// One of each text, its values filled with `Sample`s.
            pub fn samples() -> Vec<Text<'a>> {
                vec![$(Text::$name $({ $($field: <$kind as Sample>::sample()),* })?),*]
            }
        }
    };
}

/// A value to fill a text with in the tests: easy to find in what each language says.
#[cfg(test)]
pub trait Sample {
    fn sample() -> Self;
}

#[cfg(test)]
impl Sample for &str {
    fn sample() -> Self {
        "<value>"
    }
}

#[cfg(test)]
impl Sample for u64 {
    fn sample() -> Self {
        7
    }
}

#[cfg(test)]
impl Sample for usize {
    fn sample() -> Self {
        7
    }
}

#[cfg(test)]
impl Sample for u16 {
    fn sample() -> Self {
        7
    }
}

#[cfg(test)]
impl Sample for i32 {
    fn sample() -> Self {
        7
    }
}

#[cfg(test)]
impl Sample for Lang {
    fn sample() -> Self {
        Lang::En
    }
}

texts! {
    // The notification area (tray.rs)
    TraySettings,
    TrayDoNotDisturb,
    TrayQuit,

    // Commands of main.rs
    StatsClearFailed { error: &'a str },
    NoSessionToJoin,
    RelayMissing { path: &'a str },
    InstallationMissing { id: &'a str },
    ConnectorUnknown,
    ConnectFailed { error: &'a str },
    AutostartFailed { error: &'a str },

    // What an agent does (connectors/activity.rs): the verb before its target
    VerbRun,
    VerbRead,
    VerbEdit,
    VerbWrite,
    VerbBrowse,
    VerbSearch,
    VerbFetch,
    VerbDelegate,
    VerbPlan,
    VerbAsk,
    VerbUse,
    VerbThink,
    VerbWork,
    /// A tool the agent did not name.
    ToolUnnamed,

    // What an agent's session does (connectors)
    AgentStarts,
    AgentClosedSession,
    AgentFinished,
    AgentFailed,
    AgentInterrupted,
    AgentWaitsForOk,
    AgentWaitsForOkInTerminal,
    AgentAsks,
    AgentAsksInTerminal,
    AgentTakesOver,
    AgentNotification,
    /// A badge that borrows the name of an AI îlot follows.
    OtherAgent { name: &'a str },
    /// The connector of `ilot emit` and `ilot run`.
    ConnectorOther,

    // Permissions and questions (connectors/hook_events.rs, claude_code.rs)
    WantsToRun { agent: &'a str },
    WantsToEditEverywhere { agent: &'a str },
    WantsToEditFile { agent: &'a str },
    WantsToEditFiles { agent: &'a str },
    WantsToFetch { agent: &'a str },
    WantsToUse { agent: &'a str, tool: &'a str },
    ClaudeAsks { count: usize },

    // Plugging an AI in (connectors, wsl.rs)
    AmpInWsl,
    KiroInWsl,
    NothingToPlugOnWindows,
    NotWrittenByIlot { path: &'a str },
    CommandMissing { path: &'a str },
    UnsafeArgument { arg: &'a str },
    PathNotForCmd { path: &'a str },
    FileChanging { path: &'a str },
    NotAWslFile { path: &'a str },
    UnreadableLink { path: &'a str },
    UnreadableMode { path: &'a str },

    // What makes a shell command risky (risk.rs)
    RiskDelete,
    RiskGitClean,
    RiskGitResetHard,
    RiskWslUnregister,
    RiskPushForce,
    RiskPush,
    RiskPushDelete,
    RiskNetwork,
    RiskRemoteShell,
    RiskRemoteCopy,
    RiskRawSocket,
    RiskPipeToShell,
    RiskEncoded,
    RiskAdmin,
    RiskWorldWritable,
    RiskSystemFiles,
    RiskRegistry,
    RiskServices,
    RiskPower,
    RiskDisk,
    RiskSecurity,
    RiskAccounts,
    RiskTooNested,

    // « OK Mochi », Mochi's voice and the dictation (voice/, dictation.rs)
    VoicePhrasesRefused,
    VoiceFailedToStart { error: &'a str },
    /// No recognizer of the grammar's language: said, never replaced by an online service.
    VoiceNoRecognizer { lang: Lang },
    VoiceNoMicrophone,
    VoiceNotStarted,
    VoiceNotAnswering,
    VoiceSilent,
    NoVoice { lang: Lang },
    DictationUnavailable { lang: Lang },
    DictationNoMicrophone,
    AskNoKey,
    AskRefused,
    AskKeyRejected,
    AskBusy,
    AskUnreachable,
    AskUnexpected { status: u16 },
    /// What Mochi is told before the question (sent to Claude): he answers in the interface's language.
    AskSystem,
    AskContent { question: &'a str, context: &'a str },

    // The chat (chat/), its attachments (attachments.rs) and the window Mochi reads (window_context.rs)
    ChatEmptyQuestion,
    ChatBadRequest,
    ChatMessageTooLong { max: usize },
    /// Sent to the model with the question, in the language the user writes in.
    ChatContextAttached { title: &'a str, app: &'a str, text: &'a str },
    ChatFileAttached { name: &'a str, text: &'a str },
    ChatImageAttached { name: &'a str },
    /// The question of an attachment sent alone.
    ChatWhatDoYouThink,
    ChatNoClient,
    ChatKeyInvalid,
    ChatStoppedAnswering { provider: &'a str },
    ChatProviderError { provider: &'a str, message: &'a str },
    ChatMissingKey { provider: &'a str },
    ChatMalformed,
    ChatUnknownError,
    GeminiBlockedAnswer,
    GeminiBlockedQuestion,
    ProviderKeyRefused { name: &'a str },
    ProviderModelMissing { name: &'a str },
    ProviderTooLong { name: &'a str },
    ProviderLimited { name: &'a str },
    ProviderDown { name: &'a str },
    ProviderRefusedBecause { name: &'a str, status: u16, detail: &'a str },
    ProviderRefused { name: &'a str, status: u16 },
    OllamaDown,
    LmStudioDown,
    ProviderUnreachable { name: &'a str },
    AttachmentNotDropped,
    AttachmentUnknown,
    FileCannotOpen { name: &'a str },
    FileCannotRead { name: &'a str },
    FileTooBig { name: &'a str, kilobytes: u64 },
    FileNotTextOrImage { name: &'a str },
    ContextUnknown,
    ContextNotConfirmed,
    ContextNothing,
    ContextGone,
    BlockPasswordManager,
    BlockBank,
    BlockSignIn,
    BlockSystem,
    BlockPasswordField,
    BlockAdmin,

    // The keys of the vault (vault.rs, vault/credential_manager.rs)
    SecretBadName,
    KeyEmpty,
    KeyTooLong,
    KeyControl,
    KeyUnreadable,

    // E-mail (mail.rs)
    MailBadAddress { address: &'a str },
    MailTooManyRecipients { max: usize },
    MailSubjectTooLong { max: usize },
    MailTooLong,
    MailInvalid,
    /// Before « Envoyer », when no mail app is registered.
    MailYourApp,
    /// The app a `mailto:` link opened.
    MailYourMail,
    MailCannotOpen,
    MailClosedUnsent { client: &'a str },
    MailCopiedForOutlook,
    MailNoApp,
    MailDragFile { name: &'a str },
    MailTextCopied,

    // Music, videos and the windows they play in (media.rs, video.rs, players.rs, on_top.rs, source_app.rs, app_volume.rs, native_host.rs)
    MediaNoAccess,
    MediaNothingPlaying,
    MediaRefused,
    /// The name of Windows' own player.
    MediaPlayerApp,
    OtherApp,
    VideoInvalidLink,
    VideoNoWindow,
    VideoNotABar,
    VideoBarTitle,
    MiniPlayerTitle,
    VideoWindowMissing,
    WindowMoveRefused,
    AppWindowMissing,
    SpotifySilent,
    VolumeSpotifyOnly,
    ExtensionNotConnected,
    BrowserNoAnswer,

    // Messages of the other apps (notifications.rs)
    /// The app of the notifications of sites open in a browser.
    WebsitesApp,
    MessageGone,
    BrowserNotOpen,
    AppNotOpen { app: &'a str },
    WindowsSettingsMissing,

    // The island, Mochi on the desktop, the screens (window.rs, buddy.rs, screens.rs)
    MoveInvalid,
    IslandWindowMissing,
    ScreenMissing,
    ScreenNumber { n: usize },
    PositionInvalid,
    ModeUnknown { mode: &'a str },
    MochiWindowMissing,
    IslandNotPlaced,

    // Shortcuts, sounds, clipboard, report (shortcuts.rs, sounds.rs, clipboard.rs, export.rs)
    ShortcutInvalid { shortcut: &'a str },
    ShortcutWindowsKey,
    ShortcutF12,
    ShortcutNeedsModifier,
    ActionUnknown { action: &'a str },
    IslandKeepsShortcut,
    ShortcutUsedByIlot,
    ShortcutTaken { shortcut: &'a str },
    SoundUnknown,
    SoundNoFile,
    SoundTooHeavy,
    SoundUnreadable,
    SoundBusy,
    SoundsFolderMissing,
    SoundsFolderFailed { error: &'a str },
    SoundsFolderCannotOpen,
    ClipboardTooLong,
    ClipboardInvalid,
    ClipboardBusy,
    ClipboardFailed,
    /// The report image's file name when the interface gives none.
    ReportFileName,
    ReportSaveTitle,
    ReportImageFilter,
    ReportDialogClosed,
    ReportImageMissing,
    ReportImageRefused,
    SaveFailed { error: &'a str },

    // Pastilles: the main tool, the local servers, the services (pastilles/)
    PastilleReading,
    ToolClosed { tool: &'a str },
    ToolOpenOn { tool: &'a str, project: &'a str },
    ToolOpen { tool: &'a str },
    ToolNotOpen { tool: &'a str },
    ToolFrontRefused { tool: &'a str },
    NoMainTool,
    OllamaPullRefused,
    PullStopped,
    ModelNameInvalid,
    AddOllamaFirst,
    PullRunning,
    PullPreparing,
    PullDone,
    PullVerifying,
    PullDownloading,
    PullFinishing,
    LocalNotRunning { name: &'a str },
    LocalNeedsToken { name: &'a str },
    LocalOdd { name: &'a str },
    NoModelInstalled,
    NoModelLoaded { n: u64 },
    ModelLoaded { name: &'a str },
    ModelsLoaded { n: usize },
    ServiceUnreachable,
    ServiceTokenRefused,
    ServiceLimited,
    ServiceError { code: u16 },
    ServiceBadAnswer,
    ServiceNoToken,
    AccountIdMissing,
    UsernameMissing,

    // What the services say (pastilles/services/): states, counts, cards
    StateLive,
    StateFailed,
    StateDeploying,
    StateCanceled,
    StateWaiting,
    StateIdle,
    StateBlocked,
    StateDeleted,
    StateBuilding,
    StateUnknown,
    Deploying,
    CountFailed { n: usize },
    CountDeploying { n: usize },
    CountLive { n: usize },
    ToReview { n: u64 },
    ReviewsRequested { n: u64 },
    MetricToday,
    MetricUpcoming,
    MetricLive,
    MetricRunning,
    MetricFailed,
    MetricToReview,

    // Cal.com
    CalcomTokenLabel,
    CalcomRights,
    CalcomWarning,
    CalcomToday { clock: &'a str },
    CalcomTomorrow { clock: &'a str },
    CalcomOn { day: &'a str, month: &'a str, clock: &'a str },
    CalcomToConfirm,
    CalcomBooking,
    CalcomNone,
    CalcomFree,
    CalcomNext { title: &'a str, when: &'a str },
    CalcomShort { clock: &'a str },

    // Cloudflare
    CloudflareAccountLabel,
    CloudflareAccountHint,
    CloudflareTokenLabel,
    CloudflareRights,
    CloudflarePages { n: u64 },
    CloudflareWorkers { n: u64 },
    CloudflarePagesFailed,
    CloudflareWorkersMetric,
    CloudflareAlerts { n: u64 },
    CloudflareAlertsMetric,

    // Docker Hub
    DockerUserLabel,
    DockerUserHint,
    DockerTokenLabel,
    DockerRights,
    DockerTagPushed,
    DockerPrivate,
    DockerPublic,
    DockerNone,
    DockerLatest { n: u64, latest: &'a str },
    DockerImages,
    DockerPulls,
    DockerRecentTags,

    // Dokploy
    DokployTokenLabel,
    DokployRights,
    DokployWarning,
    DokployNone,
    DokployApps { n: u64 },
    DokployUnnamed,

    // GitHub, GitLab
    GithubTokenLabel,
    GithubRights,
    GithubDraft { repo: &'a str },
    GithubNoPr,
    GithubOpenPrs { n: u64 },
    GithubCiFailing { n: u64 },
    GithubShort { n: u64 },
    GithubOpenMetric,
    GithubCiMetric,
    GitlabTokenLabel,
    GitlabRights,
    GitlabNoMr,
    GitlabOpenMrs { n: u64 },
    GitlabPipelineFailed,
    GitlabPipelinesRunning { n: u64 },
    GitlabShort { n: u64 },
    GitlabOpenMetric,
    GitlabPipelinesMetric,

    // n8n, Notion, Resend, Stripe, Vercel
    N8nTokenLabel,
    N8nRights,
    N8nWarning,
    N8nNoneToday,
    N8nToday { n: u64 },
    N8nWorkflow { id: &'a str },
    N8nFailedExecution,
    N8nShort { n: usize },
    NotionTokenLabel,
    NotionRights,
    NotionUntitled,
    NotionNone,
    NotionLatest { title: &'a str },
    NotionEditedToday,
    NotionRecent,
    NotionPageEdited,
    NotionToday { n: u64, line: &'a str },
    ResendTokenLabel,
    ResendRights,
    ResendWarning,
    ResendNoneToday,
    ResendToday { n: u64 },
    ResendShortSent { n: usize },
    ResendBounces { n: u64 },
    ResendBounce,
    ResendSent,
    ResendBouncesMetric,
    ResendComplaints,
    StripeTokenLabel,
    StripeRights,
    StripeNoneToday,
    StripeToday { n: u64, amount: &'a str },
    /// An amount: « 1 234,56 € » in French, « €1,234.56 » in English.
    Money { sign: &'a str, whole: u64, cents: &'a str, symbol: &'a str },
    StripeOtherCurrencies,
    StripePayment,
    StripePaymentAt { clock: &'a str },
    StripePayments,
    StripeAmount,
    StripeDeclined,
    VercelTokenLabel,
    VercelRights,
    VercelWarning,
    VercelNone,
    VercelLatest { state: &'a str, name: &'a str },
    VercelProduction,
    VercelPreview,
    VercelLatestMetric,

    // « Aller au terminal » (terminal.rs)
    TerminalNotFound,
    TerminalNotInFront,
    TerminalTabInFront { project: &'a str },
    TerminalInFront,
    EditorOpenOn { editor: &'a str, project: &'a str },
    EditorInFront { editor: &'a str },
    ConsoleOfProjectInFront { project: &'a str },
    ConsoleInFront,

    // The Windows Credential Manager (vault/credential_manager.rs)
    /// Shown next to each key in the Credential Manager of Windows.
    CredentialComment,
    CredentialSaveFailed { code: &'a str },
    CredentialReadFailed { code: &'a str },
    CredentialRemoveFailed { code: &'a str },
    CredentialListFailed { code: &'a str },
}
