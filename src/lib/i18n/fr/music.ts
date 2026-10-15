// music: the player's surfaces (src/components/music, src/components/player), the
// mini-player (src/MiniPlayer.svelte) and the commands bar of the small video player
// (src/VideoBar.svelte).

export const music = {
  transport: {
    group: "Commandes de lecture",
    previous: "Morceau précédent",
    play: "Lire",
    pause: "Mettre en pause",
    next: "Morceau suivant",
  },
  modes: {
    group: "Modes de lecture",
    shuffle: "Lecture aléatoire",
    shuffleOn: "Aléatoire : activé",
    shuffleOff: "Aléatoire",
    /** `state`: the name of the repeat mode (src/lib/media.ts, `repeatLabel`). */
    repeat: (state: string) => `Répétition : ${state.toLowerCase()}`,
    repeatTitle: (state: string, next: string) => `${state}. Clic : ${next.toLowerCase()}`,
  },
  position: "Position dans le morceau",
  progress: "Progression",
  nothingPlaying: "rien en cours",
  volume: {
    label: "Volume",
    level: (percent: number) => `${percent} %`,
    app: (app: string) => `Volume de ${app}`,
    appTitle: (app: string) => `Volume de ${app} seul, gardé par Windows comme dans son mélangeur`,
  },
  bubble: {
    /** `what`: the title and the artist. */
    playing: (what: string) => `En lecture : ${what}`,
    open: (what: string) => `En lecture : ${what}. Ouvrir le lecteur`,
    hint: "Clique pour ouvrir, tire pour détacher",
  },
  live: "En direct",
  newTrack: "Nouveau morceau",
  youtube: {
    label: "Lien YouTube",
    placeholder: "Colle un lien YouTube",
    play: "Lire",
    invalid: "Ce lien ne mène pas à une vidéo YouTube.",
  },
  dock: "Ramener dans l'île",
  mini: {
    window: "Mini-lecteur",
    label: "Mini-lecteur détaché",
    unavailable: "Lecteur indisponible",
  },
  videoBar: {
    label: "Commandes de la vidéo",
    inIlot: "Dans îlot",
    close: "Fermer la vidéo",
  },
};
