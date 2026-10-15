// agentsPanel: the « Agents » tab, its cards and the decisions journal
// (src/lib/i18n/i18n.svelte.ts).

const s = (n: number): string => (n > 1 ? "s" : "");

export const agentsPanel = {
  /** Around the `.avant-ilot` code. */
  safe: { before: "Rien n'est branché sans ton clic. Avant toute modification, la config de l'IA est sauvegardée (", after: ")." },
  followed: "Suivis seulement",
  followedCount: (n: number) => `${n} agents`,
  followedLead: "îlot montre ce qu'ils font. Leurs autorisations se donnent toujours dans leur terminal.",
  desktop:
    "Son onglet Code lance Claude Code, avec les mêmes hooks : la carte Claude le suit déjà, permissions et questions comprises. Le chat n'envoie aucun événement : rien à suivre.",
  badgeTitle: "Un autre agent",
  /** Around the `ilot_agent` and `docs/AGENTS.md` codes. */
  badge: {
    add: "Ajoute",
    middle: "à la charge utile de ses hooks pour lui donner son nom et sa couleur, dans la pilule et le hub (voir",
    end: ").",
  },
  card: {
    other: "Autre agent",
    searching: "Recherche…",
    noWsl: "Aucune distribution WSL trouvée.",
    notInstalled: (name: string) => `${name} n'est installé ni sous Windows ni dans WSL.`,
    pluggedIn: (places: readonly string[]) => `Branché dans ${places.join(" et ")}`,
    installedIn: (places: readonly string[]) => `Installé dans ${places.join(" et ")}`,
    notes: {
      claudeRepair: "Rallume l'interrupteur pour refaire sa connexion : ses questions s'afficheront alors dans l'île.",
      repair: "Sa connexion est à réparer : rallume l'interrupteur pour la refaire.",
      codexHooks: "À faire dans Codex : tape /hooks et approuve les hooks d'îlot.",
      reload: (name: string) => `Relance ${name} pour qu'il charge le plugin d'îlot.`,
      geminiTerminal: "Gemini demande ses autorisations dans son terminal : réponds-y directement.",
      inTerminal: (name: string) => `${name} attend ta réponse dans son terminal : réponds-y directement.`,
      claudeTrust: "Claude ignore les hooks d'un dossier que tu n'as pas encore approuvé.",
      cursor: "Cursor lance aussi les hooks de Claude : îlot les compte pour Cursor, une seule fois.",
      followedOnly: "Suivi d'activité seulement : ses autorisations se donnent dans son terminal, jamais par îlot.",
      other: "Pour tout autre outil : ilot emit et ilot run -- commande dans WSL, ou ilot-cli.exe sous Windows.",
    },
    wait: "Un instant…",
    disconnect: "Déconnecter",
    connect: "Connecter",
    connectName: (name: string) => `Connecter ${name}`,
    steps: (name: string) => `Dernières actions de ${name}`,
    /** `name` null: the generic card, for any other tool. */
    noAction: (name: string | null) => `Aucune action pour l'instant. Lance une session de ${name ?? "ton outil"} : elle apparaîtra ici.`,
    busy: (share: number) => `Actif ${share} % de l'heure`,
    sessions: (n: number, tools: string) => `, ${n} sessions (${tools})`,
    reachTip: (project: string) => `Ramène au premier plan la fenêtre de ${project}`,
    reach: "Aller au terminal",
  },
  journal: {
    title: "Journal des décisions",
    count: (n: number) => `${n} décision${s(n)}`,
    less: "Afficher moins",
    all: (n: number) => `Tout afficher (${n})`,
    empty: "Aucune décision pour l'instant. Chaque demande que tu acceptes ou refuses s'inscrira ici.",
  },
};
