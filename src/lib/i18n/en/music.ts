// music, in English: the same keys as ../fr/music.ts.

import type { Catalog } from "../fr";

export const music: Catalog["music"] = {
  transport: {
    group: "Playback controls",
    previous: "Previous track",
    play: "Play",
    pause: "Pause",
    next: "Next track",
  },
  modes: {
    group: "Playback modes",
    shuffle: "Shuffle",
    shuffleOn: "Shuffle: on",
    shuffleOff: "Shuffle",
    repeat: (state) => `Repeat mode: ${state.toLowerCase()}`,
    repeatTitle: (state, next) => `${state}. Click: ${next.toLowerCase()}`,
  },
  position: "Playback position",
  progress: "Progress",
  nothingPlaying: "nothing playing",
  volume: {
    label: "Volume",
    level: (percent) => `${percent}%`,
    app: (app) => `${app} volume`,
    appTitle: (app) => `${app}'s own volume, kept by Windows as in its volume mixer`,
  },
  bubble: {
    playing: (what) => `Now playing: ${what}`,
    open: (what) => `Now playing: ${what}. Open the player`,
    hint: "Click to open, drag to detach",
  },
  live: "Live",
  newTrack: "New track",
  youtube: {
    label: "YouTube link",
    placeholder: "Paste a YouTube link",
    play: "Play",
    invalid: "This link doesn't lead to a YouTube video.",
  },
  dock: "Return to the island",
  mini: {
    window: "Mini player",
    label: "Detached mini player",
    unavailable: "Player unavailable",
  },
  videoBar: {
    label: "Video controls",
    inIlot: "In îlot",
    close: "Close video",
  },
};
