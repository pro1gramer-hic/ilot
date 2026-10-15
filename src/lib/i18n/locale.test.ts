import { describe, expect, it } from "vitest";
import { previewLocale, resolveLocale } from "./locale";

describe("the interface's language", () => {
  it("is the one chosen in the settings", () => {
    expect(resolveLocale("en", ["fr-FR"])).toBe("en");
    expect(resolveLocale("fr", ["en-US"])).toBe("fr");
  });

  it("follows Windows when left on auto, English when îlot does not speak it", () => {
    expect(resolveLocale("auto", ["fr-FR", "en-US"])).toBe("fr");
    expect(resolveLocale("auto", ["de-DE", "en-GB"])).toBe("en");
    expect(resolveLocale("auto", ["de-DE"])).toBe("en");
    expect(resolveLocale("auto", [])).toBe("en");
  });

  it("can be forced in the browser preview's address", () => {
    expect(previewLocale("?state=hub&lang=en")).toBe("en");
    expect(previewLocale("?lang=de")).toBeNull();
    expect(previewLocale("")).toBeNull();
  });
});
