// translate, in English: the same keys as ../fr/translate.ts.

import type { Catalog } from "../fr";

const s = (n: number): string => (n === 1 ? "" : "s");

export const translate: Catalog["translate"] = {
  button: "Translate",
  pending: "Translating…",
  showOriginal: "Show original",
  showTranslation: "Show translation",
  retry: "Try again",
  addKey: "Add a DeepL key",
  addKeyHint: "Translating needs your DeepL key, added in Settings › Chat.",
  detected: (language) => `Original language: ${language}`,
  failures: {
    noKey: "Add your DeepL key in Settings › Chat.",
    keyRefused: "DeepL refused your key. Check it in Settings › Chat.",
    quota: "DeepL quota used up for this month.",
    busy: "DeepL is getting too many requests. Try again in a moment.",
    unavailable: "DeepL isn't answering right now. Try again later.",
    network: "Can't reach DeepL. Check your connection.",
    tooLong: "Text too long: 30,000 characters at most.",
    empty: "Nothing to translate.",
    badLanguage: "DeepL doesn't offer this language. Pick another one in Settings › Chat.",
    unexpected: "Unexpected answer from DeepL. Try again.",
  },
  clipboard: {
    title: "Clipboard",
    into: (language) => `Translate into ${language}`,
    lead: "This is what will go to DeepL:",
    chars: (n, count) => `${n} character${s(count)}`,
    send: "Translate",
    cancel: "Cancel",
    sending: "Translating…",
    copy: "Copy",
    copied: "Copied",
    close: "Close",
    addKey: "Add the key",
    noKey: "Add your DeepL key to translate the clipboard.",
    empty: "The clipboard holds no text.",
    tooLong: "The clipboard holds more than 30,000 characters: too long to translate.",
    busy: "Another app is holding the clipboard. Try again.",
  },
  settings: {
    title: "Translation",
    intro:
      "A Translate button sits by received messages and chat answers. The text goes to DeepL only when you click, with your key.",
    key: "DeepL key",
    keyHint: "Create your key at deepl.com/your-account/keys, then paste it here. A free key ends with :fx.",
    copyLink: "Copy link",
    linkCopied: "Link copied",
    privacy:
      "With DeepL API Free, DeepL may keep your texts for a while to improve its service. With DeepL API Pro, it deletes them after translating.",
    target: "Target language",
    targetHint: "Messages, answers and the clipboard are translated into this language.",
    followInterface: (language) => `The interface's (${language})`,
  },
};
