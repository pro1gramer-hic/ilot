import { describe, expect, it } from "vitest";
import { terminalToast } from "./terminal";

describe("« Aller au terminal »", () => {
  it("says honestly how close it got", () => {
    expect(terminalToast({ found: true, exact: true, message: "VS Code ouvert sur epilot." })).toMatchObject({ tone: "ok", title: "VS Code ouvert sur epilot." });
    expect(terminalToast({ found: true, exact: false, message: "Terminal mis au premier plan, onglet non identifié." })).toMatchObject({
      tone: "acc",
      title: "Terminal mis au premier plan, onglet non identifié.",
    });
    expect(terminalToast({ found: false, exact: false, message: "Aucune fenêtre de terminal trouvée pour cette session." }).tone).toBe("err");
  });
});
