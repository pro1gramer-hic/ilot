// timeline: the agents' time (src/lib/timeline.ts, report.ts, report-image.ts): the journal
// of each AI, durations, and the Monday report with its image. Clock times, day names and
// numbers come from Intl, in the language of the moment.

const s = (n: number): string => (n > 1 ? "s" : "");

/** « 1er » for the first of the month. */
const dayOfMonth = (date: Date): string => (date.getUTCDate() === 1 ? "1er" : String(date.getUTCDate()));
const month = (date: Date): string => new Intl.DateTimeFormat("fr", { month: "long", timeZone: "UTC" }).format(date);

export const timeline = {
  /** One line of an AI's journal when a session changes. */
  steps: {
    waiting: (task: string) => `Attend ton accord : ${task}`,
    done: (task: string) => `A terminé : ${task}`,
    failed: (task: string) => `A échoué : ${task}`,
    paused: (task: string) => `En pause : ${task}`,
  },
  duration: {
    lessThanMinute: "moins d'une minute",
    /** « 4 min ». */
    minutes: (n: number) => `${n} min`,
    /** « 1 h 05 »: `minutes` has its two digits. */
    hours: (hours: number, minutes: string) => `${hours} h ${minutes}`,
    /** « 45 s ». */
    seconds: (n: number) => `${n} s`,
  },
  report: {
    /** « du 5 au 11 octobre 2026 », « du 28 septembre au 4 octobre 2026 » (UTC dates). */
    weekRange: (start: Date, end: Date): string => {
      const sameYear = start.getUTCFullYear() === end.getUTCFullYear();
      const from =
        start.getUTCMonth() === end.getUTCMonth() && sameYear
          ? dayOfMonth(start)
          : `${dayOfMonth(start)} ${month(start)}${sameYear ? "" : ` ${start.getUTCFullYear()}`}`;
      return `du ${from} au ${dayOfMonth(end)} ${month(end)} ${end.getUTCFullYear()}`;
    },
    codeTime: "Temps de code",
    agentsAtWork: "agents au travail",
    sessions: "Sessions",
    files: "Fichiers touchés",
    lines: "Lignes",
    commands: "Commandes lancées",
    permissions: "Permissions",
    /** `count` is already written as a number of the language. */
    allowed: (n: number, count: string) => `${count} autorisée${s(n)}`,
    denied: (n: number, count: string) => `${count} refusée${s(n)}`,
    mainAgent: "Agent principal",
    mainProject: "Projet principal",
    busiestDay: "Jour le plus chargé",
    longest: "Plus longue session",
    none: "Aucun",
    noSession: "Aucune",
    /** « Projet A », when the user hides the projects' names. */
    project: (letter: string) => `Projet ${letter}`,
    projectHidden: "Projet masqué",
    /** The image's suggested file name, ASCII only. */
    imageName: (monday: string) => `Bilan ilot ${monday}`,
    imageTitle: "Ma semaine avec îlot",
    imageHours: "de code avec mes agents",
    imageFooter: (sessions: string) => `${sessions} sessions comptées sur mon PC, par îlot`,
    drawFailed: "Dessin impossible.",
    imageFailed: "Image impossible à créer.",
    cleared: { title: "Statistiques effacées", sub: "Le bilan repart de zéro." },
    ready: { title: "Ton bilan de la semaine est prêt", sub: "Clique sur l'île pour le voir" },
  },
};
