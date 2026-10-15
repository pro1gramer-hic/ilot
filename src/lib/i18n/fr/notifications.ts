// notifications: what the island says on its own (src/lib/notifications.ts, island/banners.ts):
// the messages' banners and threads, the quick replies, the finished agents and the toasts.

export const notifications = {
  /** « Discord · 3 nouveaux messages », after the app's name. */
  grouped: (n: number) => ` · ${n} nouveaux messages`,
  /** Before the sender of a masked message. */
  newFrom: "Nouveau message de ",
  /** After the sender: the colon follows French typography. */
  said: (text: string) => `\u00a0: ${text}`,
  masked: "Contenu masqué",
  noMessage: "Aucun message",
  /** The user's own reply, last line of a thread. */
  mine: (text: string) => `Toi\u00a0: ${text}`,
  /** Three short answers in the tone of each app. */
  quickReplies: {
    whatsapp: ["J'arrive vers 18h", "Parfait, merci !", "Je te rappelle"],
    discord: ["Je regarde ce soir", "Envoie le lien", "Pas avant demain"],
    snapchat: ["Je regarde plus tard", "Haha", "Je t'écris après"],
    teams: ["J'arrive", "Je regarde ça", "On en parle après"],
    slack: ["Je regarde ça", "Merci !", "Je reviens vers toi"],
    gmail: ["Bien reçu, merci", "Je regarde ça", "Je te réponds vite"],
    other: ["D'accord", "Merci !", "Je te réponds plus tard"],
  } as Record<"whatsapp" | "discord" | "snapchat" | "teams" | "slack" | "gmail" | "other", readonly [string, string, string]>,
  replyCopied: (app: string, opened: boolean) =>
    opened ? `Réponse copiée\u00a0: colle-la dans ${app}` : `Réponse copiée\u00a0: ouvre ${app} et colle-la`,
  writeFirst: "Écris une réponse d'abord.",
  notCopied: (reason: string) => `Réponse non copiée\u00a0: ${reason}`,
  /** The e-mail draft handed to the mail app. */
  mailOpened: (client: string | null) => (client === null ? "Message ouvert dans ta messagerie" : `Message ouvert dans ${client}`),
  mailCheck: "Vérifie-le, puis envoie-le de là.",
  finished: (name: string, project: string, duration: string) => `${name} a terminé · ${project} · ${duration}`,
  finishedMany: (n: number) => `${n} agents ont terminé`,
  allowed: { title: "Commande autorisée", sub: (tool: string) => `${tool} peut continuer.` },
  denied: { title: "Commande refusée", sub: (tool: string) => `${tool} est prévenu.` },
  always: { title: "Toujours autorisée", sub: (detail: string) => `Seulement pour : ${detail}` },
  inTerminal: { title: "Réponds dans le terminal", sub: (tool: string) => `${tool} y attend ta réponse.` },
  answered: { title: "Réponse envoyée", sub: (tool: string) => `${tool} continue avec ta réponse.` },
  dropped: {
    title: (file: string) => `${file} : chemin copié`,
    sub: "Colle-le dans le terminal de ton agent. îlot n'a pas ouvert le fichier.",
  },
};
