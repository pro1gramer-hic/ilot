// agents: the words of the agent sessions (src/lib/agents.ts, hub.ts, questions.ts), shared
// by the pill, the hub, the panel and the timeline.

const s = (n: number): string => (n > 1 ? "s" : "");

export const agents = {
  /** A generic tool (`ilot emit`) without a name of its own. */
  generic: { name: "Autre", tool: "Outil maison" },
  status: {
    active: "actif",
    waiting: "attend ton OK",
    paused: "en pause",
    done: "terminé",
    failed: "échec",
  },
  subagents: (n: number) => `${n} sous-agent${s(n)}`,
  /** « 3 sous-agents : Explore ×2, Plan ». */
  subagentsOf: (count: string, kinds: string) => `${count} : ${kinds}`,
  calm: "Tout est calme",
  atWork: (n: number, detail: string) => `${n} agent${s(n)} au travail · ${detail}`,
  waitsOn: (name: string, project: string, inTerminal: boolean) =>
    `${name} attend ton OK sur ${project}${inTerminal ? ", dans son terminal" : ""}`,
  nobody: "Personne ne travaille",
  /** « Claude travaille sur epilot, Gemini sur site ». */
  worksOn: (name: string, project: string, first: boolean) => `${name} ${first ? "travaille sur" : "sur"} ${project}`,
  tickerWaiting: "attend ton accord",
  tickerWorking: (project: string) => `travaille sur ${project}`,
  /** What an agent does, by the verb the core sends (src-tauri/src/connectors/activity.rs). */
  verbs: {
    run: "lance",
    read: "lit",
    edit: "édite",
    write: "écrit",
    browse: "parcourt",
    search: "cherche",
    fetch: "consulte",
    delegate: "délègue",
    plan: "planifie",
    ask: "demande",
    use: "utilise",
    think: "réfléchit",
    work: "travaille",
  },
  decisions: {
    allow: "Autorisée",
    always: "Toujours autorisée",
    deny: "Refusée",
    suspended: "Refusée (suspension)",
    terminal: "Répondue dans le terminal",
    expired: "Expirée",
  },
  /** An AI on its card. */
  state: {
    repair: "À réparer",
    absent: "Non installé",
    off: "Déconnecté",
    suspended: "Suspendu",
    waiting: "Attend ton accord",
    working: "Travail en cours",
    failed: "Échec",
    done: "Terminé",
    idle: "En veille",
  },
  /** Mochi's mood in the panel. */
  mood: {
    suspended: "Suspendu",
    waiting: "Attend ton accord",
    failed: "Un agent a échoué",
    fine: "Tout roule",
  },
  /** A session in the hub. */
  hub: {
    work: "travail en cours",
    perm: "attend ton accord",
    err: "en échec",
    ok: "terminé",
    play: "en lecture",
    idle: "au repos",
  },
  suspended: "Agents suspendus",
  working: (n: number) => `${n} agent${s(n)} au travail`,
  noneWorking: "Aucun agent au travail",
  /** The four steps of a session followed live. */
  liveSteps: ["Lire", "Éditer", "Commande", "Terminé"],
  /** « Question 2 sur 3 ». */
  questionStep: (step: number, total: number) => `Question ${step} sur ${total}`,
};
