// translate: DeepL in îlot (src/lib/translate.ts, stores/translate.svelte.ts): the
// « Traduire » button of messages and chat answers, the clipboard's translation in the
// island, and the Traduction section of Réglages › Chat. Language names come from Intl, in
// lower case in French (« anglais »).

const s = (n: number): string => (n > 1 ? "s" : "");

export const translate = {
  /** The button by a message or an answer, in its four states. */
  button: "Traduire",
  pending: "Traduction…",
  showOriginal: "Voir l'original",
  showTranslation: "Voir la traduction",
  retry: "Réessayer",
  /** Without a key, the button offers to add one (it opens Réglages › Chat). */
  addKey: "Ajouter une clé DeepL",
  addKeyHint: "Traduire demande ta clé DeepL, à ajouter dans Réglages › Chat.",
  /** Under a translated text. */
  detected: (language: string) => `Langue d'origine : ${language}`,
  /** What the core answered instead of a translation (src-tauri/src/translate.rs). */
  failures: {
    noKey: "Ajoute ta clé DeepL dans Réglages › Chat.",
    keyRefused: "DeepL refuse ta clé. Vérifie-la dans Réglages › Chat.",
    quota: "Quota DeepL atteint pour ce mois.",
    busy: "DeepL reçoit trop de demandes. Réessaie dans un moment.",
    unavailable: "DeepL ne répond pas pour l'instant. Réessaie plus tard.",
    network: "Impossible de joindre DeepL. Vérifie ta connexion.",
    tooLong: "Texte trop long : 30 000 caractères au plus.",
    empty: "Rien à traduire.",
    badLanguage: "DeepL ne propose pas cette langue. Choisis-en une autre dans Réglages › Chat.",
    unexpected: "Réponse inattendue de DeepL. Réessaie.",
  },
  /** « Traduire le presse-papiers », in the island. */
  clipboard: {
    title: "Presse-papiers",
    into: (language: string) => `Traduire en ${language}`,
    lead: "Voici ce qui partira chez DeepL :",
    chars: (n: string, count: number) => `${n} caractère${s(count)}`,
    send: "Traduire",
    cancel: "Annuler",
    sending: "Traduction en cours…",
    copy: "Copier",
    copied: "Copié",
    close: "Fermer",
    addKey: "Ajouter la clé",
    noKey: "Ajoute ta clé DeepL pour traduire le presse-papiers.",
    empty: "Le presse-papiers ne contient pas de texte.",
    tooLong: "Le presse-papiers contient plus de 30 000 caractères : trop long pour une traduction.",
    busy: "Une autre app tient le presse-papiers. Réessaie.",
  },
  /** The Traduction section of Réglages › Chat. */
  settings: {
    title: "Traduction",
    intro:
      "Un bouton Traduire accompagne les messages reçus et les réponses du chat. Le texte part chez DeepL seulement quand tu cliques, avec ta clé.",
    key: "Clé DeepL",
    keyHint: "Crée ta clé sur deepl.com/your-account/keys, puis colle-la ici. Une clé gratuite finit par :fx.",
    copyLink: "Copier le lien",
    linkCopied: "Lien copié",
    privacy:
      "Avec DeepL API Free, DeepL peut garder tes textes un temps pour améliorer son service. Avec DeepL API Pro, il les efface après la traduction.",
    target: "Langue d'arrivée",
    targetHint: "Les messages, les réponses et le presse-papiers sont traduits dans cette langue.",
    followInterface: (language: string) => `Celle de l'interface (${language})`,
  },
};
