import { describe, expect, it } from "vitest";
import { blocks, spans } from "./chat-text";

describe("an answer's text", () => {
  it("keeps bold and code within a line, and leaves unclosed marks as typed", () => {
    expect(spans("Le **lancement** est `reporté`.")).toEqual([
      { text: "Le " },
      { text: "lancement", bold: true },
      { text: " est " },
      { text: "reporté", code: true },
      { text: "." },
    ]);
    expect(spans("2 * 3 ** 4 et `oups")).toEqual([{ text: "2 * 3 ** 4 et `oups" }]);
  });

  it("splits paragraphs, list items, titles and fenced code", () => {
    const text = "## Résumé\nDeux lignes\nqui se suivent.\n\n- un\n* deux\n2) trois\n```ts\nconst a = 1;\n```\nFin";
    expect(blocks(text)).toEqual([
      { kind: "title", spans: [{ text: "Résumé" }] },
      { kind: "p", spans: [{ text: "Deux lignes qui se suivent." }] },
      { kind: "item", mark: "•", spans: [{ text: "un" }] },
      { kind: "item", mark: "•", spans: [{ text: "deux" }] },
      { kind: "item", mark: "2.", spans: [{ text: "trois" }] },
      { kind: "code", text: "const a = 1;" },
      { kind: "p", spans: [{ text: "Fin" }] },
    ]);
  });

  it("shows code still streaming as code", () => {
    expect(blocks("Voici :\n```\nnpm test")).toEqual([
      { kind: "p", spans: [{ text: "Voici :" }] },
      { kind: "code", text: "npm test" },
    ]);
  });

  it("never turns an answer into markup: tags stay text", () => {
    const text = '<img src=x onerror="alert(1)"> **<b>gras</b>**';
    const all = blocks(text).flatMap((b) => (b.kind === "code" ? [b.text] : b.spans.map((s) => s.text)));
    expect(all.join("")).toBe('<img src=x onerror="alert(1)"> <b>gras</b>');
  });
});
