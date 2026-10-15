// The French texts: the reference catalog, one file per part of the interface.

import { common } from "./common";
import { island } from "./island";
import { hub } from "./hub";
import { music } from "./music";
import { messages } from "./messages";
import { panel } from "./panel";
import { today } from "./today";
import { agentsPanel } from "./agentsPanel";
import { settings } from "./settings";
import { wardrobe } from "./wardrobe";
import { pastilles } from "./pastilles";
import { report } from "./report";
import { agents } from "./agents";
import { mochi } from "./mochi";
import { themes } from "./themes";
import { modes } from "./modes";
import { sounds } from "./sounds";
import { notifications } from "./notifications";
import { media } from "./media";
import { timeline } from "./timeline";
import { voice } from "./voice";
import { buddy } from "./buddy";
import { translate } from "./translate";

export const fr = {
  common,
  island,
  hub,
  music,
  messages,
  panel,
  today,
  agentsPanel,
  settings,
  wardrobe,
  pastilles,
  report,
  agents,
  mochi,
  themes,
  modes,
  sounds,
  notifications,
  media,
  timeline,
  voice,
  buddy,
  translate,
};

/** The shape every language follows. */
export type Catalog = typeof fr;
