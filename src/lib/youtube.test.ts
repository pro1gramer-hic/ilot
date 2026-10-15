import { describe, expect, it } from "vitest";
import { youtubeStart, youtubeVideoId } from "./youtube";

const ID = "dQw4w9WgXcQ";

describe("youtubeVideoId", () => {
  it("reads every usual link", () => {
    for (const link of [
      `https://www.youtube.com/watch?v=${ID}`,
      `https://youtube.com/watch?v=${ID}&list=PL123&index=2`,
      `https://m.youtube.com/watch?v=${ID}`,
      `https://music.youtube.com/watch?v=${ID}`,
      `https://youtu.be/${ID}?si=abc`,
      `https://www.youtube.com/shorts/${ID}`,
      `https://www.youtube.com/embed/${ID}?start=4`,
      `https://www.youtube-nocookie.com/embed/${ID}`,
      `https://www.youtube.com/live/${ID}?feature=share`,
      `youtube.com/watch?v=${ID}`,
      `  ${ID}  `,
    ]) {
      expect(youtubeVideoId(link), link).toBe(ID);
    }
  });

  it("refuses other sites and broken ids", () => {
    for (const link of [
      "",
      "bonjour",
      `https://evil.com/watch?v=${ID}`,
      `https://youtube.com.evil.com/watch?v=${ID}`,
      "https://www.youtube.com/watch?v=short",
      `https://www.youtube.com/watch?v=${ID}"><script>`,
      "https://www.youtube.com/@LofiGirl",
      `javascript:alert(1)//youtube.com/watch?v=${ID}`,
      `ftp://youtube.com/watch?v=${ID}`,
    ]) {
      expect(youtubeVideoId(link), link).toBeNull();
    }
  });
});

describe("youtubeStart", () => {
  it("reads seconds and h/m/s times", () => {
    expect(youtubeStart(`https://youtu.be/${ID}?t=90`)).toBe(90);
    expect(youtubeStart(`https://youtu.be/${ID}?t=90s`)).toBe(90);
    expect(youtubeStart(`https://www.youtube.com/watch?v=${ID}&t=1m30s`)).toBe(90);
    expect(youtubeStart(`https://www.youtube.com/watch?v=${ID}&t=1h2m3s`)).toBe(3723);
    expect(youtubeStart(`https://www.youtube.com/embed/${ID}?start=42`)).toBe(42);
  });

  it("starts at the beginning otherwise", () => {
    expect(youtubeStart(ID)).toBe(0);
    expect(youtubeStart(`https://youtu.be/${ID}`)).toBe(0);
    expect(youtubeStart(`https://youtu.be/${ID}?t=abc`)).toBe(0);
  });
});
