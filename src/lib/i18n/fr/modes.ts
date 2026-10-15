// modes: the display modes of the « Modes » tab (src/lib/modes.ts): each one's name and
// its options.

export const modes = {
  music: { label: "Musique dans l'île repliée", ticker: "Titre défilant", disc: "Disque", equalizer: "Égaliseur", none: "Rien" },
  agents: { label: "Agents dans la pilule", counter: "Compteur", dots: "Points seuls", activity: "Activité" },
  messages: { label: "Messages", preview: "Aperçu", sender: "Expéditeur seul", badge: "Pastille" },
  done: { label: "Fin de tâche", banner: "Bannière", quiet: "Discret" },
  mochi: { label: "Mochi hors de l'île", wanders: "Se balade", follows: "Te suit", stays: "Reste dans l'île", desktop: "Sur le bureau" },
  relief: { label: "Relief", deep: "3D", light: "Léger", flat: "Plat" },
  position: { label: "Position", free: "Libre", magnet: "Aimantée" },
};
