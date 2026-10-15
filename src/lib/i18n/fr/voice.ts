// voice: « OK Mochi » (src/lib/voice.ts, stores/voice.svelte.ts): the phrases Windows
// listens for in this language, what Mochi answers, and what a question to Claude carries.

const s = (n: number): string => (n > 1 ? "s" : "");

export const voice = {
  /** The phrases of each command (the recognizer knows these and nothing else), and Mochi's
   * answer once it is done. */
  commands: {
    toggleIsland: { phrases: ["ouvre l'île"], reply: "Voilà l'île." },
    goToAlert: { phrases: ["montre l'alerte", "qu'est-ce qui m'attend"], reply: "Voilà ce qui t'attend." },
    openChat: { phrases: ["ouvre le chat"], reply: "J'ouvre le chat." },
    goToTerminal: { phrases: ["va au terminal"], reply: "Je t'emmène au terminal." },
    nextPastille: { phrases: ["pastille suivante"], reply: "Pastille suivante." },
    mute: { phrases: ["coupe le son"], reply: "Je coupe le son." },
    mochiToDesktop: { phrases: ["va sur le bureau"], reply: "J'y vais." },
    openWardrobe: { phrases: ["ouvre la garde-robe"], reply: "Voici ma garde-robe." },
    translateClipboard: { phrases: ["traduis le presse-papiers"], reply: "Je regarde le presse-papiers." },
    report: { phrases: ["montre le bilan"], reply: "Voici ton bilan de la semaine." },
    stop: { phrases: ["arrête d'écouter"], reply: "J'arrête d'écouter." },
  },
  /** The question for Claude, and the phrases that ask it. */
  summary: { phrases: ["résume l'activité", "quoi de neuf"], question: "Résume mon activité." },
  /** What a question to Claude says of each agent. */
  status: {
    active: "au travail",
    waiting: "attend une réponse",
    paused: "en pause",
    done: "a terminé",
    failed: "a échoué",
  },
  agentLine: (tool: string, status: string, project: string) => `${tool} ${status} sur ${project}`,
  agentsSeparator: " ; ",
  noAgent: "aucun agent ouvert",
  agents: (list: string) => `Agents : ${list}.`,
  today: (finished: number, failures: number, decisions: number) =>
    `Aujourd'hui : ${finished} tâche${s(finished)} terminée${s(finished)}, ${failures} échec${s(failures)}, ${decisions} décision${s(decisions)}.`,
  micOff: { title: "Micro coupé", sub: "« OK Mochi » se rallume dans Réglages, Raccourcis." },
  yes: "Oui ?",
  notUnderstood: "Je n'ai pas compris.",
  cannotYet: "Je ne sais pas encore faire ça.",
  clickToSend: "Clique sur Envoyer et je demande à Claude.",
  noClaudeKey: "Je n'ai pas ta clé Claude. Ajoute-la dans les réglages du chat, puis redemande-moi.",
};
