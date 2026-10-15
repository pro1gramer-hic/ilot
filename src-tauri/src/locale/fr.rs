//! French, the reference: the texts îlot has always shown, to the character (the tests
//! check them).

use super::Text as T;
use super::{Lang, Text};

/// Every text of the core in this language.
pub fn text(text: Text<'_>) -> String {
    match text {
        // The notification area (tray.rs)
        T::TraySettings => "Réglages".into(),
        T::TrayDoNotDisturb => "Ne pas déranger".into(),
        T::TrayQuit => "Quitter".into(),

        // Commands of main.rs
        T::StatsClearFailed { error } => format!("Effacement impossible : {error}"),
        T::NoSessionToJoin => "Aucune session à rejoindre.".into(),
        T::RelayMissing { path } => format!("Relais introuvable : {path}"),
        T::InstallationMissing { id } => format!("Installation introuvable : {id}"),
        T::ConnectorUnknown => "Connecteur inconnu".into(),
        T::ConnectFailed { error } => format!("Échec : {error}"),
        T::AutostartFailed { error } => format!("Démarrage avec Windows : {error}"),

        // What an agent does (connectors/activity.rs): the verb before its target
        T::VerbRun => "lance".into(),
        T::VerbRead => "lit".into(),
        T::VerbEdit => "édite".into(),
        T::VerbWrite => "écrit".into(),
        T::VerbBrowse => "parcourt".into(),
        T::VerbSearch => "cherche".into(),
        T::VerbFetch => "consulte".into(),
        T::VerbDelegate => "délègue".into(),
        T::VerbPlan => "planifie".into(),
        T::VerbAsk => "demande".into(),
        T::VerbUse => "utilise".into(),
        T::VerbThink => "réfléchit".into(),
        T::VerbWork => "travaille".into(),
        T::ToolUnnamed => "outil".into(),

        // What an agent's session does (connectors)
        T::AgentStarts => "démarre".into(),
        T::AgentClosedSession => "a fermé la session".into(),
        T::AgentFinished => "a terminé".into(),
        T::AgentFailed => "a échoué".into(),
        T::AgentInterrupted => "interrompu".into(),
        T::AgentWaitsForOk => "attend ton OK".into(),
        T::AgentWaitsForOkInTerminal => "attend ton OK dans son terminal".into(),
        T::AgentAsks => "te pose une question".into(),
        T::AgentAsksInTerminal => "te pose une question dans son terminal".into(),
        T::AgentTakesOver => "reprend la main".into(),
        T::AgentNotification => "notification".into(),
        T::OtherAgent { name } => format!("{name} (autre)"),
        T::ConnectorOther => "Autre".into(),

        // Permissions and questions (connectors/hook_events.rs, claude_code.rs)
        T::WantsToRun { agent } => format!("{agent} veut exécuter une commande"),
        T::WantsToEditEverywhere { agent } => format!("{agent} veut modifier un fichier partout où le texte apparaît"),
        T::WantsToEditFile { agent } => format!("{agent} veut modifier un fichier"),
        T::WantsToEditFiles { agent } => format!("{agent} veut modifier des fichiers"),
        T::WantsToFetch { agent } => format!("{agent} veut consulter une page"),
        T::WantsToUse { agent, tool } => format!("{agent} veut utiliser {tool}"),
        T::ClaudeAsks { count } => if count == 1 { "Claude te pose une question".into() } else { format!("Claude te pose {count} questions") },

        // Plugging an AI in (connectors, wsl.rs)
        T::AmpInWsl => "Amp tourne dans WSL : branche-le dans ta distribution.".into(),
        T::KiroInWsl => "Kiro se branche dans WSL seulement pour l'instant.".into(),
        T::NothingToPlugOnWindows => "Rien à brancher sous Windows : ilot-cli.exe est livré avec îlot.".into(),
        T::NotWrittenByIlot { path } => format!("{path} existe déjà et n'a pas été écrit par îlot : renomme-le d'abord."),
        T::CommandMissing { path } => format!("Commande introuvable : {path}"),
        T::UnsafeArgument { arg } => format!("argument non sûr : {arg}"),
        T::PathNotForCmd { path } => format!("chemin non pris en charge par cmd : {path}"),
        T::FileChanging { path } => format!("{path} change en ce moment : réessaie dans un instant."),
        T::NotAWslFile { path } => format!("pas un fichier de WSL : {path}"),
        T::UnreadableLink { path } => format!("lien illisible : {path}"),
        T::UnreadableMode { path } => format!("droits illisibles : {path}"),

        // What makes a shell command risky (risk.rs)
        T::RiskDelete => "Supprime des fichiers de façon définitive.".into(),
        T::RiskGitClean => "Supprime les fichiers que git ne suit pas, sans retour possible.".into(),
        T::RiskGitResetHard => "Efface les modifications non commitées, sans retour possible.".into(),
        T::RiskWslUnregister => "Supprime une distribution WSL entière.".into(),
        T::RiskPushForce => "Pousse sur GitHub en forçant : l'historique distant sera écrasé.".into(),
        T::RiskPush => "Publie tes commits sur le dépôt distant.".into(),
        T::RiskPushDelete => "Supprime une branche du dépôt distant.".into(),
        T::RiskNetwork => "Accède au réseau.".into(),
        T::RiskRemoteShell => "Ouvre une session sur une autre machine.".into(),
        T::RiskRemoteCopy => "Copie des fichiers vers ou depuis une autre machine.".into(),
        T::RiskRawSocket => "Ouvre une connexion réseau directe.".into(),
        T::RiskPipeToShell => "Exécute un script téléchargé sans te le montrer.".into(),
        T::RiskEncoded => "Exécute une commande encodée, illisible ici.".into(),
        T::RiskAdmin => "Demande les droits administrateur.".into(),
        T::RiskWorldWritable => "Rend des fichiers modifiables par tout le monde.".into(),
        T::RiskSystemFiles => "Touche aux fichiers du système.".into(),
        T::RiskRegistry => "Modifie le registre de Windows.".into(),
        T::RiskServices => "Arrête, lance ou change des services du système.".into(),
        T::RiskPower => "Éteint ou redémarre l'ordinateur.".into(),
        T::RiskDisk => "Peut effacer un disque entier.".into(),
        T::RiskSecurity => "Change une protection du système (pare-feu, antivirus, règles d'exécution).".into(),
        T::RiskAccounts => "Change les comptes de l'ordinateur.".into(),
        T::RiskTooNested => "Trop imbriquée pour être vérifiée en entier.".into(),

        // « OK Mochi », Mochi's voice and the dictation (voice/, dictation.rs)
        T::VoicePhrasesRefused => "Liste de phrases refusée.".into(),
        T::VoiceFailedToStart { error } => format!("La reconnaissance vocale n'a pas démarré : {error}"),
        T::VoiceNoRecognizer { lang } => match lang {
            Lang::Fr => "Aucun moteur de reconnaissance vocale français n'est installé sur ce PC.".into(),
            Lang::En => "Aucun moteur de reconnaissance vocale anglais n'est installé sur ce PC.".into(),
        },
        T::VoiceNoMicrophone => "Micro inaccessible. Vérifie Paramètres Windows › Confidentialité › Microphone.".into(),
        T::VoiceNotStarted => "La reconnaissance vocale n'a pas démarré.".into(),
        T::VoiceNotAnswering => "La reconnaissance vocale ne répond pas.".into(),
        T::VoiceSilent => "La voix de Mochi ne répond pas.".into(),
        T::NoVoice { lang } => match lang {
            Lang::Fr => "Aucune voix française n'est installée sur ce PC.".into(),
            Lang::En => "Aucune voix anglaise n'est installée sur ce PC.".into(),
        },
        T::DictationUnavailable { lang } => match lang {
            Lang::Fr => "Dictée indisponible : la reconnaissance vocale française de Windows n'est pas installée.".into(),
            Lang::En => "Dictée indisponible : la reconnaissance vocale anglaise de Windows n'est pas installée.".into(),
        },
        T::DictationNoMicrophone => "Micro inaccessible. Autorise les applications de bureau dans Paramètres › Confidentialité › \
            Microphone.".into(),
        T::AskNoKey => "Je n'ai pas ta clé Claude. Ajoute-la dans les réglages du chat, puis redemande-moi.".into(),
        T::AskRefused => "Claude n'a pas voulu répondre à ça.".into(),
        T::AskKeyRejected => "Claude refuse ta clé. Vérifie-la dans les réglages du chat.".into(),
        T::AskBusy => "Claude est très demandé. Réessaie dans un moment.".into(),
        T::AskUnreachable => "Je n'arrive pas à joindre Claude. Vérifie ta connexion.".into(),
        T::AskUnexpected { status } => format!("Claude a répondu une erreur ({status})."),
        T::AskSystem => "Tu es Mochi, la petite mascotte d'îlot, une île en haut de l'écran qui suit les agents de code \
            de l'utilisateur. On t'a parlé à voix haute et ta réponse sera lue par une voix de synthèse : \
            réponds en français, en deux ou trois phrases courtes et naturelles, sans liste, sans titre ni \
            mise en forme. Appuie-toi seulement sur les données fournies ; s'il manque une information, \
            dis-le simplement.".into(),
        T::AskContent { question, context } => format!("{question}\n\nDonnées d'îlot :\n{context}"),

        // The chat (chat/), its attachments (attachments.rs) and the window Mochi reads (window_context.rs)
        T::ChatEmptyQuestion => "Écris une question".into(),
        T::ChatBadRequest => "Demande invalide".into(),
        T::ChatMessageTooLong { max } => format!("Message trop long : {max} caractères au plus."),
        T::ChatContextAttached { title, app, text } => format!("Contexte joint par l'utilisateur : la fenêtre « {title} » ({app}).\n\"\"\"\n{text}\n\"\"\""),
        T::ChatFileAttached { name, text } => format!("Fichier joint : {name}\n\"\"\"\n{text}\n\"\"\""),
        T::ChatImageAttached { name } => format!("Image jointe : {name}"),
        T::ChatWhatDoYouThink => "Qu'en dis-tu ?".into(),
        T::ChatNoClient => "Client réseau indisponible".into(),
        T::ChatKeyInvalid => "Clé invalide : enregistre-la à nouveau".into(),
        T::ChatStoppedAnswering { provider } => format!("{provider} ne répond plus."),
        T::ChatProviderError { provider, message } => format!("{provider} : {message}"),
        T::ChatMissingKey { provider } => format!("Ajoute ta clé {provider} dans Réglages › Chat."),
        T::ChatMalformed => "Réponse illisible du fournisseur".into(),
        T::ChatUnknownError => "erreur inconnue".into(),
        T::GeminiBlockedAnswer => "Gemini a bloqué la réponse (filtre de sécurité de Google)".into(),
        T::GeminiBlockedQuestion => "Gemini a bloqué la question (filtre de sécurité de Google)".into(),
        T::ProviderKeyRefused { name } => format!("Clé refusée par {name}. Vérifie-la dans Réglages › Chat."),
        T::ProviderModelMissing { name } => format!("Modèle introuvable chez {name}. Choisis-en un autre."),
        T::ProviderTooLong { name } => format!("Message trop long pour {name}."),
        T::ProviderLimited { name } => format!("{name} limite tes demandes (quota ou crédit). Réessaie plus tard."),
        T::ProviderDown { name } => format!("{name} est surchargé ou en panne. Réessaie dans un moment."),
        T::ProviderRefusedBecause { name, status, detail } => format!("{name} a refusé la demande ({status}) : {detail}"),
        T::ProviderRefused { name, status } => format!("{name} a refusé la demande ({status})."),
        T::OllamaDown => "Ollama ne répond pas sur ce PC. Lance Ollama, puis réessaie.".into(),
        T::LmStudioDown => "LM Studio ne répond pas. Démarre son serveur local (port 1234), puis réessaie.".into(),
        T::ProviderUnreachable { name } => format!("Impossible de joindre {name}. Vérifie ta connexion Internet."),
        T::AttachmentNotDropped => "Ce fichier n'a pas été déposé sur l'île".into(),
        T::AttachmentUnknown => "Pièce jointe introuvable : dépose le fichier à nouveau".into(),
        T::FileCannotOpen { name } => format!("Impossible d'ouvrir {name}"),
        T::FileCannotRead { name } => format!("Impossible de lire {name}"),
        T::FileTooBig { name, kilobytes } => format!("{name} est trop gros pour le chat ({kilobytes} Ko au plus)."),
        T::FileNotTextOrImage { name } => format!("Mochi ne sait lire que du texte et des images (PNG, JPEG, GIF, WebP) : {name} n'en est pas."),
        T::ContextUnknown => "Cette fenêtre n'est plus proposée : relâche Mochi dessus à nouveau".into(),
        T::ContextNotConfirmed => "Le texte de cette fenêtre n'a pas été joint au chat".into(),
        T::ContextNothing => "Mochi n'a trouvé aucun texte lisible dans cette fenêtre".into(),
        T::ContextGone => "Cette fenêtre est fermée ou ne répond pas".into(),
        T::BlockPasswordManager => "Gestionnaire de mots de passe : îlot ne lit pas cette fenêtre.".into(),
        T::BlockBank => "Banque ou paiement : îlot ne lit pas cette fenêtre.".into(),
        T::BlockSignIn => "Page de connexion ou code de sécurité : îlot ne lit pas cette fenêtre.".into(),
        T::BlockSystem => "Fenêtre de sécurité de Windows : îlot ne la lit pas.".into(),
        T::BlockPasswordField => "Cette fenêtre contient un champ de mot de passe : îlot ne la lit pas.".into(),
        T::BlockAdmin => "Programme lancé en administrateur : Windows empêche îlot de lire cette fenêtre.".into(),

        // The keys of the vault (vault.rs, vault/credential_manager.rs)
        T::SecretBadName => "Nom de secret refusé".into(),
        T::KeyEmpty => "La clé est vide".into(),
        T::KeyTooLong => "Clé trop longue : 2 560 octets au plus".into(),
        T::KeyControl => "La clé contient des caractères invisibles".into(),
        T::KeyUnreadable => "Clé illisible : enregistre-la à nouveau".into(),

        // E-mail (mail.rs)
        T::MailBadAddress { address } => format!("Adresse invalide : {address}"),
        T::MailTooManyRecipients { max } => format!("{max} destinataires au plus"),
        T::MailSubjectTooLong { max } => format!("Objet trop long : {max} caractères au plus"),
        T::MailTooLong => "Message trop long".into(),
        T::MailInvalid => "Message invalide".into(),
        T::MailYourApp => "ton application de messagerie".into(),
        T::MailYourMail => "ta messagerie".into(),
        T::MailCannotOpen => "Impossible d'ouvrir la messagerie".into(),
        T::MailClosedUnsent { client } => format!("Message fermé dans {client} sans envoi."),
        T::MailCopiedForOutlook => "Objet et texte copiés : colle-les dans Outlook.".into(),
        T::MailNoApp => "Aucune application de messagerie n'a répondu.".into(),
        T::MailDragFile { name } => format!("Glisse {name} dans le message : son dossier est ouvert."),
        T::MailTextCopied => "Texte trop long pour le lien : il est copié en entier.".into(),

        // Music, videos and the windows they play in (media.rs, video.rs, players.rs, on_top.rs, source_app.rs, app_volume.rs, native_host.rs)
        T::MediaNoAccess => "Windows refuse l'accès aux médias en cours. Redémarre îlot pour réessayer.".into(),
        T::MediaNothingPlaying => "Aucun média en cours.".into(),
        T::MediaRefused => "L'app n'a pas accepté la commande.".into(),
        T::MediaPlayerApp => "Lecteur multimédia".into(),
        T::OtherApp => "Autre app".into(),
        T::VideoInvalidLink => "Ce lien ne mène pas à une vidéo YouTube.".into(),
        T::VideoNoWindow => "Aucune vidéo n'est ouverte.".into(),
        T::VideoNotABar => "Cette fenêtre ne commande aucune vidéo.".into(),
        T::VideoBarTitle => "Commandes de la vidéo · îlot".into(),
        T::MiniPlayerTitle => "Mini-lecteur · îlot".into(),
        T::VideoWindowMissing => "Impossible de trouver la fenêtre de cette vidéo.".into(),
        T::WindowMoveRefused => "Windows n'a pas laissé îlot déplacer cette fenêtre.".into(),
        T::AppWindowMissing => "Impossible de trouver la fenêtre de cette app.".into(),
        T::SpotifySilent => "Spotify ne joue pas de son en ce moment.".into(),
        T::VolumeSpotifyOnly => "Volume réglable pour Spotify seulement.".into(),
        T::ExtensionNotConnected => "L'extension îlot du navigateur n'est pas branchée.".into(),
        T::BrowserNoAnswer => "Le navigateur n'a pas répondu.".into(),

        // Messages of the other apps (notifications.rs)
        T::WebsitesApp => "Sites web".into(),
        T::MessageGone => "Ce message n'est plus disponible.".into(),
        T::BrowserNotOpen => "Le navigateur n'est pas ouvert.".into(),
        T::AppNotOpen { app } => format!("{app} n'est pas ouvert."),
        T::WindowsSettingsMissing => "Paramètres Windows introuvables.".into(),

        // The island, Mochi on the desktop, the screens (window.rs, buddy.rs, screens.rs)
        T::MoveInvalid => "Déplacement invalide".into(),
        T::IslandWindowMissing => "Fenêtre de l'île introuvable".into(),
        T::ScreenMissing => "Écran introuvable".into(),
        T::ScreenNumber { n } => format!("Écran {n}"),
        T::PositionInvalid => "Position invalide".into(),
        T::ModeUnknown { mode } => format!("Mode inconnu : {mode}"),
        T::MochiWindowMissing => "Fenêtre de Mochi introuvable".into(),
        T::IslandNotPlaced => "L'île n'est pas encore placée".into(),

        // Shortcuts, sounds, clipboard, report (shortcuts.rs, sounds.rs, clipboard.rs, export.rs)
        T::ShortcutInvalid { shortcut } => format!("Raccourci invalide : {shortcut}"),
        T::ShortcutWindowsKey => "La touche Windows est réservée à Windows.".into(),
        T::ShortcutF12 => "F12 est réservée par Windows au débogage.".into(),
        T::ShortcutNeedsModifier => "Ajoute Ctrl ou Alt : seule, cette touche ne marcherait plus ailleurs.".into(),
        T::ActionUnknown { action } => format!("Action inconnue : {action}"),
        T::IslandKeepsShortcut => "L'île garde toujours un raccourci : c'est par lui qu'elle s'ouvre au clavier.".into(),
        T::ShortcutUsedByIlot => "Ce raccourci sert déjà à une autre action d'îlot.".into(),
        T::ShortcutTaken { shortcut } => format!("{shortcut} est déjà pris par Windows ou une autre app. L'ancien raccourci reste."),
        T::SoundUnknown => "Son inconnu".into(),
        T::SoundNoFile => "Aucun fichier pour ce son".into(),
        T::SoundTooHeavy => "Fichier trop lourd (2 Mo au plus)".into(),
        T::SoundUnreadable => "Son illisible".into(),
        T::SoundBusy => "Son occupé".into(),
        T::SoundsFolderMissing => "Dossier des sons introuvable".into(),
        T::SoundsFolderFailed { error } => format!("Dossier des sons : {error}"),
        T::SoundsFolderCannotOpen => "Impossible d'ouvrir le dossier des sons".into(),
        T::ClipboardTooLong => "Texte trop long pour le presse-papiers.".into(),
        T::ClipboardInvalid => "Ce texte ne peut pas aller dans le presse-papiers.".into(),
        T::ClipboardBusy => "Le presse-papiers est occupé par une autre app. Réessaie.".into(),
        T::ClipboardFailed => "Windows n'a pas accepté la copie.".into(),
        T::ReportFileName => "Bilan îlot".into(),
        T::ReportSaveTitle => "Enregistrer le bilan".into(),
        T::ReportImageFilter => "Image PNG".into(),
        T::ReportDialogClosed => "La fenêtre d'enregistrement s'est fermée.".into(),
        T::ReportImageMissing => "Image manquante.".into(),
        T::ReportImageRefused => "Image refusée : ce n'est pas le bilan.".into(),
        T::SaveFailed { error } => format!("Enregistrement impossible : {error}"),

        // Pastilles: the main tool, the local servers, the services (pastilles/)
        T::PastilleReading => "Lecture en cours…".into(),
        T::ToolClosed { tool } => format!("{tool} n'est pas ouvert"),
        T::ToolOpenOn { tool, project } => format!("{tool} ouvert sur {project}"),
        T::ToolOpen { tool } => format!("{tool} est ouvert"),
        T::ToolNotOpen { tool } => format!("{tool} n'est pas ouvert."),
        T::ToolFrontRefused { tool } => format!("Windows n'a pas laissé îlot afficher {tool}."),
        T::NoMainTool => "Aucun outil principal choisi.".into(),
        T::OllamaPullRefused => "Ollama n'a pas accepté le téléchargement.".into(),
        T::PullStopped => "Le téléchargement s'est arrêté.".into(),
        T::ModelNameInvalid => "Nom de modèle invalide (exemple : gemma4, llama3.2:3b).".into(),
        T::AddOllamaFirst => "Ajoute d'abord la pastille Ollama.".into(),
        T::PullRunning => "Un téléchargement est déjà en cours.".into(),
        T::PullPreparing => "Préparation".into(),
        T::PullDone => "Terminé".into(),
        T::PullVerifying => "Vérification".into(),
        T::PullDownloading => "Téléchargement".into(),
        T::PullFinishing => "Finalisation".into(),
        T::LocalNotRunning { name } => format!("{name} n'est pas lancé"),
        T::LocalNeedsToken { name } => format!("{name} demande un jeton : îlot ne lit que sans authentification"),
        T::LocalOdd { name } => format!("{name} répond de façon inattendue"),
        T::NoModelInstalled => "Aucun modèle installé".into(),
        T::NoModelLoaded { n } => format!("Aucun modèle en mémoire, {}", plural(n, "installé", "installés")),
        T::ModelLoaded { name } => format!("{name} en mémoire"),
        T::ModelsLoaded { n } => format!("{n} modèles en mémoire"),
        T::ServiceUnreachable => "Injoignable pour l'instant".into(),
        T::ServiceTokenRefused => "Jeton refusé : vérifie ses droits".into(),
        T::ServiceLimited => "Limite de requêtes atteinte, îlot patiente".into(),
        T::ServiceError { code } => format!("Le service répond mal (erreur {code})"),
        T::ServiceBadAnswer => "Réponse inattendue du service".into(),
        T::ServiceNoToken => "Aucun jeton : ajoute-le dans Réglages".into(),
        T::AccountIdMissing => "Identifiant du compte manquant".into(),
        T::UsernameMissing => "Nom d'utilisateur manquant".into(),

        // What the services say (pastilles/services/): states, counts, cards
        T::StateLive => "en ligne".into(),
        T::StateFailed => "en échec".into(),
        T::StateDeploying => "en déploiement".into(),
        T::StateCanceled => "annulé".into(),
        T::StateWaiting => "en attente".into(),
        T::StateIdle => "au repos".into(),
        T::StateBlocked => "bloqué".into(),
        T::StateDeleted => "supprimé".into(),
        T::StateBuilding => "en construction".into(),
        T::StateUnknown => "état inconnu".into(),
        T::Deploying => "Déploiement…".into(),
        T::CountFailed { n } => format!("{n} en échec"),
        T::CountDeploying { n } => format!("{n} en déploiement"),
        T::CountLive { n } => format!("{n} en ligne"),
        T::ToReview { n } => format!("{n} à relire"),
        T::ReviewsRequested { n } => plural(n, "relecture demandée", "relectures demandées"),
        T::MetricToday => "Aujourd'hui".into(),
        T::MetricUpcoming => "À venir".into(),
        T::MetricLive => "En ligne".into(),
        T::MetricRunning => "En cours".into(),
        T::MetricFailed => "En échec".into(),
        T::MetricToReview => "À relire".into(),

        // Cal.com
        T::CalcomTokenLabel => "Clé d'API (cal_live_…)".into(),
        T::CalcomRights => "Cal.com n'a pas de clé en lecture seule : îlot ne fait que lire les rendez-vous à venir.".into(),
        T::CalcomWarning => "Une clé Cal.com a de nombreux droits. îlot ne fait que lire tes prochains rendez-vous : \
            donne-lui une date d'expiration.".into(),
        T::CalcomToday { clock } => format!("aujourd'hui à {clock}"),
        T::CalcomTomorrow { clock } => format!("demain à {clock}"),
        T::CalcomOn { day, month, clock } => format!("le {day}/{month} à {clock}"),
        T::CalcomToConfirm => "À confirmer".into(),
        T::CalcomBooking => "Rendez-vous".into(),
        T::CalcomNone => "Aucun rendez-vous à venir".into(),
        T::CalcomFree => "Agenda libre".into(),
        T::CalcomNext { title, when } => format!("Prochain : {title}, {when}"),
        T::CalcomShort { clock } => format!("RDV {clock}"),

        // Cloudflare
        T::CloudflareAccountLabel => "Identifiant du compte".into(),
        T::CloudflareAccountHint => "32 caractères, visibles dans le tableau de bord, à droite de la page d'accueil du compte.".into(),
        T::CloudflareTokenLabel => "Jeton d'API personnalisé".into(),
        T::CloudflareRights => "Lecture seule, limitée à ce compte : Cloudflare Pages (Read), Workers Scripts (Read), \
            Notifications (Read).".into(),
        T::CloudflarePages { n } => plural(n, "projet Pages", "projets Pages"),
        T::CloudflareWorkers { n } => plural(n, "Worker", "Workers"),
        T::CloudflarePagesFailed => "Pages en échec".into(),
        T::CloudflareWorkersMetric => "Workers".into(),
        T::CloudflareAlerts { n } => plural(n, "alerte", "alertes"),
        T::CloudflareAlertsMetric => "Alertes 24 h".into(),

        // Docker Hub
        T::DockerUserLabel => "Nom d'utilisateur Docker Hub".into(),
        T::DockerUserHint => "Celui du jeton : îlot liste les images de ce compte.".into(),
        T::DockerTokenLabel => "Jeton d'accès personnel (dckr_pat_…)".into(),
        T::DockerRights => "Repo Read-only (ou Public Repo Read-only si tout est public).".into(),
        T::DockerTagPushed => "Tag poussé".into(),
        T::DockerPrivate => "Image privée".into(),
        T::DockerPublic => "Image publique".into(),
        T::DockerNone => "Aucune image".into(),
        T::DockerLatest { n, latest } => format!("{}, dernière mise à jour : {latest}", plural(n, "image", "images")),
        T::DockerImages => "Images".into(),
        T::DockerPulls => "Téléchargements".into(),
        T::DockerRecentTags => "Tags récents".into(),

        // Dokploy
        T::DokployTokenLabel => "Clé d'API (Profil, section API/CLI)".into(),
        T::DokployRights => "Dokploy ne propose pas de clé en lecture seule : îlot ne fait que lire.".into(),
        T::DokployWarning => "La clé Dokploy donne tous les droits du compte. îlot ne fait que des lectures, mais garde-la \
            pour îlot seul et donne-lui une date d'expiration.".into(),
        T::DokployNone => "Aucune application trouvée".into(),
        T::DokployApps { n } => plural(n, "application", "applications"),
        T::DokployUnnamed => "Sans nom".into(),

        // GitHub, GitLab
        T::GithubTokenLabel => "Jeton d'accès fin (fine-grained)".into(),
        T::GithubRights => "Lecture seule : Pull requests et Commit statuses en lecture, sur les dépôts choisis.".into(),
        T::GithubDraft { repo } => format!("{repo}, brouillon"),
        T::GithubNoPr => "Aucune PR ouverte".into(),
        T::GithubOpenPrs { n } => plural(n, "PR ouverte", "PR ouvertes"),
        T::GithubCiFailing { n } => format!("CI en échec sur {n}"),
        T::GithubShort { n } => format!("{n} PR"),
        T::GithubOpenMetric => "PR ouvertes".into(),
        T::GithubCiMetric => "CI en échec".into(),
        T::GitlabTokenLabel => "Jeton d'accès personnel".into(),
        T::GitlabRights => "Portée read_api seulement (lecture de l'API).".into(),
        T::GitlabNoMr => "Aucune MR ouverte".into(),
        T::GitlabOpenMrs { n } => plural(n, "MR ouverte", "MR ouvertes"),
        T::GitlabPipelineFailed => "dernier pipeline en échec".into(),
        T::GitlabPipelinesRunning { n } => plural(n, "pipeline en cours", "pipelines en cours"),
        T::GitlabShort { n } => format!("{n} MR"),
        T::GitlabOpenMetric => "MR ouvertes".into(),
        T::GitlabPipelinesMetric => "Pipelines en échec".into(),

        // n8n, Notion, Resend, Stripe, Vercel
        T::N8nTokenLabel => "Clé d'API (Settings, n8n API)".into(),
        T::N8nRights => "Sur Enterprise : portées execution:read seulement. Sinon la clé a tous les droits : donne-lui \
            une date d'expiration.".into(),
        T::N8nWarning => "Hors Enterprise, une clé n8n donne tous les droits du compte. îlot ne fait que lire les exécutions.".into(),
        T::N8nNoneToday => "Aucune exécution aujourd'hui".into(),
        T::N8nToday { n } => format!("{} aujourd'hui", plural(n, "exécution", "exécutions")),
        T::N8nWorkflow { id } => format!("Workflow {id}"),
        T::N8nFailedExecution => "Exécution en échec".into(),
        T::N8nShort { n } => format!("{n} exéc."),
        T::NotionTokenLabel => "Secret de connexion interne".into(),
        T::NotionRights => "Capacité « Read content » seule, sans information sur les utilisateurs. Partage ensuite les \
            pages voulues avec la connexion.".into(),
        T::NotionUntitled => "Sans titre".into(),
        T::NotionNone => "Aucune page partagée avec îlot".into(),
        T::NotionLatest { title } => format!("Dernière modifiée : {title}"),
        T::NotionEditedToday => "Modifiées aujourd'hui".into(),
        T::NotionRecent => "Récentes".into(),
        T::NotionPageEdited => "Page modifiée".into(),
        T::NotionToday { n, line } => format!("{} aujourd'hui. {line}", plural(n, "page modifiée", "pages modifiées")),
        T::ResendTokenLabel => "Clé d'API (re_…)".into(),
        T::ResendRights => "Resend n'a pas de clé en lecture seule : lister les e-mails demande sans doute une clé « Full \
            access ».".into(),
        T::ResendWarning => "Attention : Resend ne propose aucune clé en lecture seule. Une clé « Full access » peut aussi \
            envoyer et supprimer. îlot ne fait que lire, mais crée une clé pour îlot seul.".into(),
        T::ResendNoneToday => "Aucun e-mail envoyé aujourd'hui".into(),
        T::ResendToday { n } => format!("{} aujourd'hui", plural(n, "e-mail envoyé", "e-mails envoyés")),
        T::ResendShortSent { n } => format!("{n} envoyés"),
        T::ResendBounces { n } => plural(n, "rebond", "rebonds"),
        T::ResendBounce => "Rebond".into(),
        T::ResendSent => "Envoyés".into(),
        T::ResendBouncesMetric => "Rebonds".into(),
        T::ResendComplaints => "Plaintes".into(),
        T::StripeTokenLabel => "Clé restreinte (rk_live_…)".into(),
        T::StripeRights => "Clé restreinte : PaymentIntents en lecture (Read), tout le reste sur None.".into(),
        T::StripeNoneToday => "Aucun paiement aujourd'hui".into(),
        T::StripeToday { n, amount } => format!("{} aujourd'hui, {amount}", plural(n, "paiement", "paiements")),
        T::Money { sign, whole, cents, symbol } => {
            let cents = if cents.is_empty() { String::new() } else { format!(",{cents}") };
            format!("{sign}{}{cents}\u{a0}{symbol}", grouped(whole, '\u{202f}'))
        },
        T::StripeOtherCurrencies => "(et d'autres devises)".into(),
        T::StripePayment => "Paiement".into(),
        T::StripePaymentAt { clock } => format!("Paiement de {clock}"),
        T::StripePayments => "Paiements".into(),
        T::StripeAmount => "Montant du jour".into(),
        T::StripeDeclined => "Refusés".into(),
        T::VercelTokenLabel => "Jeton d'accès".into(),
        T::VercelRights => "Portée la plus étroite possible : un seul projet, ou une équipe.".into(),
        T::VercelWarning => "Vercel ne propose pas de jeton en lecture seule : îlot ne fait que lire, mais limite sa portée à \
            un projet.".into(),
        T::VercelNone => "Aucun déploiement".into(),
        T::VercelLatest { state, name } => format!("Dernier déploiement {state} : {name}"),
        T::VercelProduction => "Production".into(),
        T::VercelPreview => "Aperçu".into(),
        T::VercelLatestMetric => "Derniers".into(),

        // « Aller au terminal » (terminal.rs)
        T::TerminalNotFound => "Aucune fenêtre de terminal trouvée pour cette session.".into(),
        T::TerminalNotInFront => "Windows n'a pas laissé îlot passer la fenêtre au premier plan.".into(),
        T::TerminalTabInFront { project } => format!("Windows Terminal mis au premier plan, sur l'onglet du projet {project}."),
        T::TerminalInFront => "Terminal mis au premier plan, onglet non identifié.".into(),
        T::EditorOpenOn { editor, project } => format!("{editor} ouvert sur {project}."),
        T::EditorInFront { editor } => format!("{editor} mis au premier plan, fenêtre du projet non identifiée."),
        T::ConsoleOfProjectInFront { project } => format!("Terminal du projet {project} mis au premier plan."),
        T::ConsoleInFront => "Terminal mis au premier plan, fenêtre non identifiée.".into(),

        // The Windows Credential Manager (vault/credential_manager.rs)
        T::CredentialComment => "Clé gardée par îlot".into(),
        T::CredentialSaveFailed { code } => format!("Gestionnaire d'identification Windows : enregistrement impossible ({code})"),
        T::CredentialReadFailed { code } => format!("Gestionnaire d'identification Windows : lecture impossible ({code})"),
        T::CredentialRemoveFailed { code } => format!("Gestionnaire d'identification Windows : suppression impossible ({code})"),
        T::CredentialListFailed { code } => format!("Gestionnaire d'identification Windows : liste impossible ({code})"),
    }
}

/// « 1 PR ouverte », « 3 PR ouvertes »: plural from two on.
fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n > 1 { many } else { one })
}

/// « 1 234 567 »: thousands apart by `separator`.
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
