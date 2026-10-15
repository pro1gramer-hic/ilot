// report: « Bilan de la semaine », in the « Aujourd'hui » tab (src/lib/i18n/i18n.svelte.ts).

export const report = {
  weeks: { last: "Semaine passée", current: "Cette semaine" },
  week: "Semaine",
  back: "Aujourd'hui",
  counting: "Je fais les comptes…",
  emptyTitle: "Pas encore de bilan pour cette semaine",
  emptyText: "îlot compte le temps de tes agents à partir de maintenant. Reviens lundi : le bilan sera prêt.",
  title: (range: string) => `Ta semaine ${range}`,
  /** After the week's coding time, in bold. */
  hero: "de code avec tes agents",
  days: "Temps de code par jour",
  chart: "Temps de code de chaque jour, le plus chargé en couleur",
  day: (day: string, time: string) => `${day} : ${time}`,
  byAgent: "Par agent",
  share: (time: string, percent: number) => `${time}, ${percent} %`,
  hideProjects: "Masquer les noms de projets",
  exported: {
    drawing: "Image en préparation…",
    saved: "Image enregistrée.",
    cancelled: "Enregistrement annulé.",
    error: "L'image n'a pas pu être enregistrée.",
  },
  export: "Exporter l'image",
  local: "Les compteurs restent sur ton PC : des nombres et des noms de projets, jamais de commande ni de message.",
  confirmClear: "Confirmer l'effacement",
  clear: "Effacer mes statistiques",
};
