// Records each scene of the demo from îlot's browser preview (fake data only) as JPEG
// frames with their timestamps, through Edge's screencast at twice the pixel density. Run from the repository root
// while `npm run dev -- --port 5180` serves the preview:
//   node video/scripts/record.mjs
// then `node video/scripts/encode.mjs` turns each scene into a clip for Remotion.

import { chromium } from "@playwright/test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";

const ROOT = "video/public/raw";
const URL = "http://localhost:5180/";

const browser = await chromium.launch({ channel: "msedge", args: ["--force-device-scale-factor=2"] });
const page = await browser.newPage({ viewport: { width: 980, height: 720 }, deviceScaleFactor: 2 });
const cdp = await page.context().newCDPSession(page);

const show = (state) => page.evaluate((s) => window.__ilot.showState(s), state);

async function scene(name, start, play) {
  await page.goto(`${URL}?state=${start}`);
  await page.waitForTimeout(2500);
  const dir = `${ROOT}/${name}`;
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const frames = [];
  const onFrame = async ({ data, metadata, sessionId }) => {
    const file = `f${String(frames.length).padStart(5, "0")}.jpg`;
    frames.push({ file, t: metadata.timestamp });
    writeFileSync(`${dir}/${file}`, Buffer.from(data, "base64"));
    await cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
  };
  cdp.on("Page.screencastFrame", onFrame);
  await cdp.send("Page.startScreencast", { format: "jpeg", quality: 95, maxWidth: 1960, maxHeight: 1440 });
  await play();
  await cdp.send("Page.stopScreencast");
  await page.waitForTimeout(300);
  cdp.off("Page.screencastFrame", onFrame);
  writeFileSync(`${dir}/frames.json`, JSON.stringify(frames));
  console.log(name, frames.length, "frames");
}

await scene("salut", "pill", async () => {
  await show("salut");
  await page.waitForTimeout(3500);
});

await scene("agents", "pill", async () => {
  await page.waitForTimeout(600);
  await show("pill-busy");
  await page.waitForTimeout(1800);
  await show("banner-finished");
  await page.waitForTimeout(3000);
});

await scene("live", "pill-busy", async () => {
  await page.waitForTimeout(500);
  await show("hub-live");
  await page.waitForTimeout(7000);
});

await scene("permission", "pill-busy", async () => {
  await page.waitForTimeout(500);
  await show("permission");
  await page.waitForTimeout(2200);
  await page.getByRole("button", { name: /^Autoriser/ }).first().hover();
  await page.waitForTimeout(500);
  await page.getByRole("button", { name: /^Autoriser/ }).first().click();
  await page.waitForTimeout(2500);
});

await scene("message", "pill-busy", async () => {
  await page.waitForTimeout(500);
  await show("banner-message");
  await page.waitForTimeout(2000);
  await page.getByRole("button", { name: "Répondre" }).first().click();
  await page.waitForTimeout(1600);
  await page.getByRole("button", { name: "J'arrive vers 18h" }).click();
  await page.waitForTimeout(2600);
});

await scene("music", "pill-busy", async () => {
  await page.waitForTimeout(500);
  await show("music");
  await page.waitForTimeout(3200);
  await show("pill-music");
  await page.waitForTimeout(1800);
});

await scene("pastilles", "pill-busy", async () => {
  await page.waitForTimeout(500);
  await show("hub-pastilles");
  await page.waitForTimeout(4500);
});

await browser.close();
