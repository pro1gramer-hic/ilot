// messages, in English: the same keys as ../fr/messages.ts.

import type { Catalog } from "../fr";

export const messages: Catalog["messages"] = {
  reply: {
    to: (name) => `Reply to ${name}`,
    field: (name) => `Your reply to ${name}`,
    send: "Copy and open",
    hint: (app) => `Enter copies your reply and opens ${app}, Esc cancels.`,
  },
};
