// Shows a command exactly as the agent will run it: control, bidi, invisible and
// look-alike space characters become visible labels (docs/06-SECURITE.md, "Commande
// trompeuse"). Rendering stays plain text: no HTML is built from the command.

export type Segment =
  | { kind: "text"; value: string }
  | { kind: "newline" }
  | { kind: "hidden"; label: string };

const NAMES: Record<number, string> = {
  0x09: "TAB",
  0x0d: "CR",
  0x1b: "ESC",
  0x7f: "DEL",
  0xa0: "NBSP",
  0xad: "SHY",
  0x061c: "ALM",
  0x180e: "MVS",
  0x200b: "ZWSP",
  0x200c: "ZWNJ",
  0x200d: "ZWJ",
  0x200e: "LRM",
  0x200f: "RLM",
  0x202a: "LRE",
  0x202b: "RLE",
  0x202c: "PDF",
  0x202d: "LRO",
  0x202e: "RLO",
  0x202f: "NNBSP",
  0x2060: "WJ",
  0x2066: "LRI",
  0x2067: "RLI",
  0x2068: "FSI",
  0x2069: "PDI",
  0x3000: "IDSP",
  0xfeff: "BOM",
};

/** Characters a reader cannot see, or could mistake for something else (same ranges as
 * `is_deceptive` in claude_code.rs). */
export function isDeceptive(code: number): boolean {
  return (
    (code < 0x20 && code !== 0x0a) ||
    (code >= 0x7f && code <= 0x9f) ||
    code === 0xa0 ||
    code === 0xad ||
    code === 0x034f ||
    code === 0x061c ||
    code === 0x115f ||
    code === 0x1160 ||
    code === 0x17b4 ||
    code === 0x17b5 ||
    (code >= 0x180b && code <= 0x180f) ||
    code === 0x2800 ||
    code === 0x3164 ||
    code === 0xffa0 ||
    (code >= 0x1d173 && code <= 0x1d17a) ||
    (code >= 0x2000 && code <= 0x200f) ||
    (code >= 0x2028 && code <= 0x202f) ||
    (code >= 0x205f && code <= 0x206f) ||
    code === 0x3000 ||
    (code >= 0xfe00 && code <= 0xfe0f) ||
    code === 0xfeff ||
    (code >= 0xfff9 && code <= 0xfffb) ||
    (code >= 0xe0000 && code <= 0xe007f) ||
    (code >= 0xe0100 && code <= 0xe01ef)
  );
}

function label(code: number): string {
  const hex = code.toString(16).toUpperCase().padStart(4, "0");
  return NAMES[code] ? `${NAMES[code]} U+${hex}` : `U+${hex}`;
}

export function reveal(text: string): Segment[] {
  const segments: Segment[] = [];
  let plain = "";
  const flush = (): void => {
    if (plain) segments.push({ kind: "text", value: plain });
    plain = "";
  };
  for (const char of text) {
    const code = char.codePointAt(0)!;
    if (code === 0x0a) {
      flush();
      segments.push({ kind: "newline" });
    } else if (isDeceptive(code)) {
      flush();
      segments.push({ kind: "hidden", label: label(code) });
    } else {
      plain += char;
    }
  }
  flush();
  return segments;
}

const isCombining = (code: number): boolean =>
  (code >= 0x0300 && code <= 0x036f) ||
  (code >= 0x1ab0 && code <= 0x1aff) ||
  (code >= 0x1dc0 && code <= 0x1dff) ||
  (code >= 0x20d0 && code <= 0x20ff) ||
  (code >= 0xfe20 && code <= 0xfe2f);

/** Greek and Cyrillic letters, the usual look-alikes of Latin ones. */
const isGreekOrCyrillic = (code: number): boolean => code >= 0x0370 && code <= 0x052f;

/** Same rule as `looks_spoofed` in claude_code.rs: a word that mixes Latin letters with
 * Greek or Cyrillic ones (a Cyrillic « і » in « gіthub.com »), or marks stacked on one
 * letter. */
export function looksSpoofed(text: string): boolean {
  const codes = [...text].map((char) => char.codePointAt(0)!);
  if (codes.some((code, index) => isCombining(code) && isCombining(codes[index + 1] ?? 0))) return true;
  return text
    .split(/[\s!-/:-@[-`{-~]/u)
    .some((word) => /[A-Za-z]/.test(word) && [...word].some((char) => isGreekOrCyrillic(char.codePointAt(0)!)));
}

/** True when the command holds anything that `reveal` would flag or break. */
export function hasHiddenCharacters(text: string): boolean {
  return reveal(text).some((s) => s.kind !== "text");
}
