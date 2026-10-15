// mochi: Mochi's names and words (src/lib/mochi/*, mood.ts, actions.ts, assistant.ts): his
// heads, silhouettes, colors, animations and outfits as the settings and the wardrobe name
// them, and what he says in his bubble.

const s = (n: number): string => (n > 1 ? "s" : "");

export const mochi = {
  heads: {
    idle: "Content",
    blink: "Clin d'œil",
    happy: "Heureux",
    wow: "Surpris",
    worry: "Inquiet",
    sleep: "Endormi",
    love: "Amoureux",
    star: "Émerveillé",
    wink: "Malicieux",
  },
  kinds: { meche: "Mèche", ecran: "Écran", oreilles: "Oreilles" },
  palettes: {
    lavande: "Lavande",
    peche: "Pêche",
    menthe: "Menthe",
    ciel: "Ciel",
    citron: "Citron",
    nuit: "Nuit",
    ivoire: "Ivoire",
    lagon: "Lagon",
    anis: "Anis",
    fuchsia: "Fuchsia",
    pervenche: "Pervenche",
    glycine: "Glycine",
  },
  /** The ten animations of the Mochi tab. */
  animations: {
    still: "Immobile",
    breathe: "Respiration",
    bounce: "Saut",
    rock: "Danse",
    squish: "Clic",
    dizzy: "Tournis",
    swallow: "Avaler",
    watch: "Suivre",
    fly: "Voler",
    shake: "Alerte",
  },
  slots: { head: "Tête", eyes: "Yeux", neck: "Cou", ears: "Oreilles" },
  pieces: {
    bonnet: "Bonnet",
    noel: "Bonnet de Noël",
    casquette: "Casquette",
    sorciere: "Chapeau de sorcière",
    couronne: "Couronne",
    fete: "Chapeau de fête",
    fleur: "Fleur",
    rondes: "Lunettes rondes",
    soleil: "Lunettes de soleil",
    echarpe: "Écharpe",
    noeud: "Nœud papillon",
    casque: "Casque audio",
    cacheoreilles: "Cache-oreilles",
  },
  seasons: {
    nouvelan: "Nouvel An",
    noel: "Noël",
    halloween: "Halloween",
    hiver: "Hiver",
    printemps: "Printemps",
    ete: "Été",
  },
  /** What he says when clicked, in turn. */
  pokes: [
    "Hé ! Ça chatouille !",
    "Wah ! Tu m'as fait peur.",
    "Je t'aime bien, tu sais.",
    "Allez, on retourne bosser.",
    "Tes tests vont passer, je le sens.",
    "Tu as bu de l'eau aujourd'hui ?",
    "Encore cinq minutes…",
  ],
  /** His bubble. `name` is the one the user gave him. */
  say: {
    nothingWaiting: "Rien ne t'attend pour l'instant.",
    alreadyOut: (name: string) => `${name} est déjà dehors`,
    leaving: (name: string) => `${name} part sur le bureau`,
    answerFirst: "Réponds d'abord à la demande en cours.",
    takeBreak: "Ça fait 2 h que tu travailles. Une petite pause ?",
    focusDone: "Focus terminé, bravo. Une petite pause ?",
  },
  /** His request bar (src/lib/assistant.ts). */
  assistant: {
    suggestions: { lofi: "Mets de la lo-fi", summary: "Résume le travail", focus: "Focus 25 min", quiet: "Ne pas déranger" },
    noTask: "Aucune tâche terminée aujourd'hui pour l'instant.",
    /** « Aujourd'hui : 2 tâches terminées (Claude 2) », before the rest of the summary. */
    today: (n: number, who: string) => `Aujourd'hui : ${n} tâche${s(n)} terminée${s(n)}${who ? ` (${who})` : ""}`,
    failures: (n: number) => `, ${n} échec${s(n)}`,
    lines: (added: number, removed: number, minutes: number) =>
      `, ${added} ligne${s(added)} ajoutée${s(added)} et ${removed} retirée${s(removed)}, ${minutes} min de travail d'agent.`,
    live: (list: string) => `En cours : ${list}.`,
    on: (name: string, project: string) => `${name} sur ${project}`,
  },
};
