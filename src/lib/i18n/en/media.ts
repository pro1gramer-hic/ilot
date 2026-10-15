// media, in English: the same keys as ../fr/media.ts.

import type { Catalog } from "../fr";

export const media: Catalog["media"] = {
  live: "live",
  liveTime: (elapsed) => `${elapsed} · live`,
  showVideo: "Show the video",
  open: (app) => `Open ${app}`,
  tab: (source, state) => `${source}: ${state}`,
  nothing: "nothing playing",
  playing: "playing",
  paused: "paused",
  empty: {
    spotify: { title: "Spotify isn't playing anything", hint: "Play a song in Spotify: it'll show up here." },
    youtube: {
      title: "No YouTube video",
      hintHere: "Play a video in your browser, or paste a link below.",
      hintTab: "Play a video in your browser, or paste a link in the Music tab.",
    },
    crunchyroll: { title: "No Crunchyroll episode", hint: "Play an episode: îlot will show its title and controls." },
    any: { title: "Nothing's playing", hint: "Play some music or a video: it'll show up here." },
  },
  detachPlayer: "Detach the player",
  detachVideo: "Detach the video",
  bringVideoBack: "Bring the video back",
  detachTip: "Press Ctrl+Shift+8: your browser will detach the video.",
  repeatList: "Repeat all",
  repeatTrack: "Repeat one",
  noRepeat: "No repeat",
  spotifyLine: (playing, title, artist) => `${playing ? "Playing" : "Paused"}: ${title}, ${artist}`,
};
