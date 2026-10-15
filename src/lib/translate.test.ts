import { describe, expect, it } from "vitest";
import {
  DEEPL_TARGETS,
  MAX_TRANSLATE_CHARS,
  failureOf,
  failureText,
  isTarget,
  languageName,
  languageTag,
  targetChoices,
  targetOf,
  translatable,
} from "./translate";

describe("DeepL's languages", () => {
  it("goes into the interface's language unless another one is chosen", () => {
    expect(targetOf("", "fr")).toBe("FR");
    expect(targetOf("", "en")).toBe("EN-US");
    expect(targetOf("DE", "fr")).toBe("DE");
    expect(targetOf("klingon", "en")).toBe("EN-US");
    expect(targetOf("de", "fr")).toBe("FR");
  });

  it("turns DeepL's codes into tags Intl knows", () => {
    expect(languageTag("FR")).toBe("fr");
    expect(languageTag("EN-GB")).toBe("en-GB");
    expect(languageTag("ZH-HANT")).toBe("zh-Hant");
    expect(languageTag("ES-419")).toBe("es-419");
  });

  it("names them in the interface's language", () => {
    expect(languageName("EN", "fr")).toBe("anglais");
    expect(languageName("DE", "en")).toBe("German");
    expect(languageName("PT-BR", "fr")).toMatch(/portugais/);
    expect(languageName("ZZZ", "fr")).toBe("ZZZ");
  });

  it("offers every one of them once, sorted by name", () => {
    const choices = targetChoices("fr");
    expect(choices).toHaveLength(DEEPL_TARGETS.length);
    expect(new Set(DEEPL_TARGETS).size).toBe(DEEPL_TARGETS.length);
    const names = choices.map((c) => c.name);
    expect([...names].sort(new Intl.Collator("fr").compare)).toEqual(names);
    expect(choices.find((c) => c.code === "JA")?.name).toBe("Japonais");
    expect(isTarget("EN-US") && isTarget("FR") && !isTarget("fr")).toBe(true);
  });
});

describe("what may be translated", () => {
  it("needs words, and never a hidden text", () => {
    expect(translatable("Tu rentres à quelle heure ?")).toBe(true);
    expect(translatable("Tu rentres à quelle heure ?", true)).toBe(false);
    expect(translatable("")).toBe(false);
    expect(translatable(null)).toBe(false);
    expect(translatable("  👍 18:00 !! ")).toBe(false);
    expect(translatable("x".repeat(MAX_TRANSLATE_CHARS + 1))).toBe(false);
  });

  it("says each refusal of the core, and an unknown one as unexpected", () => {
    expect(failureOf("quota")).toBe("quota");
    expect(failureOf("noKey")).toBe("noKey");
    expect(failureOf(new Error("boom"))).toBe("unexpected");
    expect(failureOf("sk-secret")).toBe("unexpected");
    expect(failureText("quota")).toBe("Quota DeepL atteint pour ce mois.");
    expect(failureText("keyRefused")).toContain("Réglages › Chat");
  });
});
