// today: the « Aujourd'hui » tab and its timeline of the last hour
// (src/lib/i18n/i18n.svelte.ts).

export const today = {
  stats: {
    finished: "Tâches terminées",
    failures: "Échecs",
    decisions: "Décisions prises",
    activeTime: "Temps actif",
  },
  activity: "Activité, dernière heure",
  activityHint: "Survole une barre ou un repère",
  running: "En cours",
  since: "depuis ",
  nobodyWorks: "Aucun agent ne travaille.",
  lastDecision: "Dernière décision",
  decidedBy: (tool: string, time: string) => `${tool}, à ${time}`,
  noDecision: "Aucune décision pour l'instant. Les demandes des agents apparaîtront ici.",
  numbers: "Aujourd'hui",
  recent: "Messages récents",
  seeAll: "Tout voir",
  hidden: (app: string) => `Contenu masqué, ${app}`,
  noMessage: "Aucun message pour l'instant.",
  week: "Bilan de la semaine",
  weekHint: "Temps de code, sessions, lignes et permissions, jour par jour. Reste sur ton PC.",
  seeWeek: "Voir le bilan",
  timeline: {
    kinds: { done: "Terminé", failed: "Échec", permission: "Permission" },
    work: "Travail",
    now: "maintenant",
    minutesAgo: (minutes: number) => `−${minutes} min`,
    bar: (label: string, minutes: string, running: boolean) => `${label}, ${minutes}${running ? ", en cours" : ""}`,
    mark: (agent: string, label: string, verdict: string | null, time: string) =>
      `${agent}, ${label}${verdict ? `, ${verdict.toLowerCase()}` : ""}, à ${time}`,
    target: (title: string, detail: string) => `${title} : ${detail}`,
    empty: "Aucune activité dans l'heure. Les agents apparaissent ici dès qu'ils travaillent.",
    targets: "Repères de la frise, à parcourir aux flèches",
    legend: "Légende",
  },
};
