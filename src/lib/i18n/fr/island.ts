// island: texts of this part of the interface (src/lib/i18n/i18n.svelte.ts): the island
// itself, its openings, its quick menu, and the permission and question cards.

export const island = {
  /** The island's region (Island.svelte), open, then folded. */
  region: "Îlot",
  regionPill: "Îlot. Clique pour ouvrir, glisse pour déplacer, Alt et flèches au clavier",
  /** The media sources' segments (SourceTabs.svelte). */
  sources: "Source",
  /** Keys written on the answers of the island's cards. */
  keyEscape: "Échap",
  keyEnter: "Entrée",
  /** A file's change, from the activity line or the end banner. */
  change: {
    title: "Voir la modification",
    label: (file: string, added: number, removed: number) =>
      `Voir la modification de ${file} : ${added} lignes ajoutées, ${removed} retirées`,
  },
  menu: {
    label: "Menu rapide",
    quiet: "Ne pas déranger",
    focus: (minutes: number) => `Focus ${minutes} min`,
    stopFocus: (left: string) => `Arrêter le focus · ${left}`,
    recenter: "Recentrer l'île",
    replayIntro: "Revoir l'intro",
    settings: "Réglages",
  },
  pill: {
    suspended: "Agents suspendus",
    working: (count: number) => `${count} agent${count > 1 ? "s" : ""} au travail`,
    none: "Aucun agent au travail",
    paused: "Suspendu",
    /** The word after the count in bold. */
    agents: (count: number) => `agent${count > 1 ? "s" : ""}`,
    idle: "En veille",
    unread: (count: number) => `${count} message${count > 1 ? "s" : ""} non lu${count > 1 ? "s" : ""}`,
  },
  event: {
    finished: (tool: string) => `${tool} a terminé`,
    agents: "Agents",
    finishedMany: (count: number) => `${count} agents ont terminé`,
    failed: (tool: string) => `${tool} a échoué`,
    failure: (project: string, detail: string) => `${project} : ${detail}`,
    view: "Voir",
    viewError: "Voir l'erreur",
  },
  drop: {
    title: "Lâche le fichier ici",
    sub: "Mochi copie son chemin pour ton agent",
  },
  file: {
    sub: "Chemin copié pour ton agent. Mochi ne l'ouvre que si tu lui poses une question.",
    ask: "Demander à Mochi",
    mail: "Joindre à un e-mail",
    later: "Plus tard",
  },
  /** Mochi in the pill: what he says, and his tooltip. */
  mochi: {
    dizzy: (name: string) => `${name} a le tournis`,
    out: (name: string) => `${name} sort de l'île`,
    tip: (name: string) => `${name} : clique, tire-le vers le bas, ou clic droit pour sa garde-robe`,
  },
  message: {
    now: (app: string) => `${app}, à l'instant`,
    masked: "Nouveau message (contenu masqué)",
    reply: "Répondre",
    open: "Ouvrir",
    read: "Marquer comme lu",
    ignore: "Ignorer",
  },
  pastille: {
    said: (name: string, line: string) => `${name} : ${line}`,
  },
  voice: {
    dot: "Le micro écoute « OK Mochi ». Couper le micro",
    dotTip: "Le micro écoute « OK Mochi ». Clic : couper",
    now: "OK Mochi, à l'instant",
    quote: (text: string) => `« ${text} »`,
    lead: "Avec ta clé, Claude recevra seulement ta demande et ceci :",
    sent: "Ce qui sera envoyé à Claude",
    send: "Envoyer à Claude",
    cancel: "Annuler",
    thinking: "Claude réfléchit…",
    repeat: "Répéter",
    close: "Fermer",
  },
  wardrobe: {
    dressed: (name: string) => `${name} dans sa tenue`,
    title: "Garde-robe",
    close: "Fermer la garde-robe (Échap)",
    closeTip: "Fermer (Échap)",
  },
  /** What the permission and question cards share. */
  request: {
    where: (tool: string, folder: string) => `${tool} dans ${folder}`,
    late: "Trop tard pour répondre ici : réponds dans le terminal de l'agent.",
    terminal: "Répondre dans le terminal",
  },
  permission: {
    dialog: "Demande de permission",
    riskHigh: "Risque élevé",
    riskLow: "Risque faible",
    body: "Demande complète",
    extras: "Paramètres en plus",
    extrasNote: "Paramètres en plus, signalés en ambre : vérifie-les avant d'autoriser.",
    spoofed: "Lettres de plusieurs alphabets mélangées : vérifie les noms de domaine et de fichiers.",
    flagged: "Plusieurs lignes ou caractères invisibles : ils sont signalés en ambre.",
    truncated: "Demande trop longue pour être affichée en entier : réponds dans le terminal.",
    readToEnd: "Lis la demande jusqu'en bas pour pouvoir l'autoriser.",
    reason: (name: string, reason: string) => `Raison donnée par ${name} : ${reason}`,
    deny: "Refuser",
    always: "Toujours pour cette commande",
    alwaysTip: "Seulement cette commande, mot pour mot, dans ce dossier, jusqu'à la fin de la session de l'agent",
    alwaysCodex: "Indisponible : Codex n'accepte pas encore de règle venue d'îlot",
    alwaysNever:
      "Indisponible : commande composée, qui lance du code du projet, ou qu'une règle ne peut pas viser exactement",
    allowed: "Autorisé",
    allow: "Autoriser",
    askedNow: "Demandé à l'instant",
    askedSeconds: (seconds: number) => `Demandé il y a ${seconds} s`,
    askedMinutes: (minutes: number) => `Demandé il y a ${minutes} min`,
    scope: (age: string, waiting: number) =>
      `« Toujours » ne vaut que pour cette commande exacte, dans ce dossier. ${age}${
        waiting > 0 ? `, ${waiting} autre${waiting > 1 ? "s" : ""} en attente` : ""
      }.`,
  },
  question: {
    dialog: "Question de l'agent",
    body: "Question complète",
    other: "Autre réponse",
    otherHint: "Autre réponse : écris la tienne",
    truncated: "Question trop longue pour être affichée en entier : réponds dans le terminal.",
    readToEnd: "Lis la question jusqu'en bas pour pouvoir répondre.",
    several: "Plusieurs choix possibles.",
    sent: "Envoyé",
    send: "Envoyer",
    next: "Suivant",
    scope: (seconds: number, name: string, waiting: number) =>
      `Sans réponse ici dans ${seconds} s, ${name} te pose la question dans le terminal.${
        waiting > 0 ? ` ${waiting} autre${waiting > 1 ? "s" : ""} demande${waiting > 1 ? "s" : ""} en attente.` : ""
      }`,
  },
};
