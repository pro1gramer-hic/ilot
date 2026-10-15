import { describe, expect, it } from "vitest";
import type { NotificationItem } from "../../lib/notifications";
import { inbox, unreadCount } from "./inbox";

const item = (id: string, receivedAt: number, count = 1): NotificationItem => ({
  id,
  app: "whatsapp",
  appLabel: "WhatsApp",
  monogram: "Wa",
  sender: "Maman",
  text: "Tu rentres à quelle heure ?",
  masked: false,
  count,
  receivedAt,
});

describe("inbox", () => {
  it("lists the shown, waiting and kept messages, newest first, each once", () => {
    const shown = item("a", 30);
    const list = inbox({ current: shown, waiting: [item("b", 50)] }, [item("c", 10), shown]);
    expect(list.map((m) => m.id)).toEqual(["b", "a", "c"]);
  });

  it("is empty when nothing came", () => {
    expect(inbox({ current: null, waiting: [] }, [])).toEqual([]);
  });

  it("counts every message of a grouped item as unread", () => {
    expect(unreadCount([item("a", 1, 3), item("b", 2)])).toBe(4);
    expect(unreadCount([])).toBe(0);
  });
});
