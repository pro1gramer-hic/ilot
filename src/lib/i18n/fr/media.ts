// media: what plays on Windows (src/lib/media.ts), as the player, the island, the
// mini-player and Spotify's pastille say it.

export const media = {
  /** A live stream has no length. */
  live: "direct",
  /** « 12:40 · direct ». */
  liveTime: (elapsed: string) => `${elapsed} · direct`,
  showVideo: "Afficher la vidéo",
  open: (app: string) => `Ouvrir ${app}`,
  /** A source tab, for screen readers and its tooltip: « Spotify : en lecture ». */
  tab: (source: string, state: string) => `${source} : ${state}`,
  nothing: "rien en cours",
  playing: "en lecture",
  paused: "en pause",
  /** The empty player, per tab: what is missing and what to do. */
  empty: {
    spotify: { title: "Spotify ne joue rien", hint: "Lance un titre dans Spotify : il s'affichera ici." },
    youtube: {
      title: "Aucune vidéo YouTube",
      hintHere: "Lance une vidéo dans ton navigateur, ou colle un lien ci-dessous.",
      hintTab: "Lance une vidéo dans ton navigateur, ou colle un lien dans l'onglet Musique.",
    },
    crunchyroll: { title: "Aucun épisode Crunchyroll", hint: "Lance un épisode : îlot affichera son titre et ses commandes." },
    any: { title: "Rien ne joue", hint: "Lance une musique ou une vidéo : elle s'affichera ici." },
  },
  detachPlayer: "Détacher le lecteur",
  detachVideo: "Détacher la vidéo",
  bringVideoBack: "Ramener la vidéo",
  detachTip: "Appuie sur Ctrl+Maj+8 : ton navigateur détachera la vidéo.",
  repeatList: "Répéter la liste",
  repeatTrack: "Répéter le titre",
  noRepeat: "Pas de répétition",
  /** Spotify's pastille: « En lecture : Midnight City, M83 ». */
  spotifyLine: (playing: boolean, title: string, artist: string) => `${playing ? "En lecture" : "En pause"} : ${title}, ${artist}`,
};
