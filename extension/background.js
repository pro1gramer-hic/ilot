/* global chrome, document */
// Service worker of îlot's extension (extension/LISEZMOI.md). It links the Crunchyroll
// tabs to îlot through native messaging (src-tauri/src/native_host.rs) and, on request,
// asks the browser to show the episode in its own floating window. Only these messages
// ever reach îlot: {type:"hello"}, {type:"state", video, pip}, {type:"pip", ok, reason?}.
// No title, no address, no picture leaves the browser.

const HOST = "io.github.hic_69.ilot";
const PROTOCOL_VERSION = 1;
const PATTERN = "https://*.crunchyroll.com/*";
const ALARM = "ilot";
/** While a Crunchyroll tab is open and îlot is away, the link is tried again this often. */
const RETRY_MINUTES = 1;
const REASONS = ["gesture", "disabled", "noVideo", "failed"];

/** @type {chrome.runtime.Port | null} */
let port = null;
/** What each Crunchyroll frame last said, by "tab:frame". */
const frames = new Map();
let sent = "";

/**
 * Runs in the Crunchyroll frames (chrome.scripting): floats the largest video, puts it
 * back, or toggles. The browser's own player shows it; a video the site marks
 * `disablePictureInPicture` is left alone, as Google's own extension does. Self-contained:
 * the browser serializes it.
 * @param {"enter" | "exit" | "toggle"} action
 */
async function floatVideo(action) {
  const floating = document.pictureInPictureElement;
  if (floating && action !== "enter") {
    await document.exitPictureInPicture();
    return { found: true, ok: true };
  }
  if (floating) return { found: true, ok: true };
  if (action === "exit") return { found: false, ok: true };
  const videos = [...document.querySelectorAll("video")].filter((video) => video.readyState > 0);
  if (videos.length === 0) return { found: false, ok: false, reason: "noVideo" };
  const allowed = videos.filter((video) => !video.disablePictureInPicture);
  if (!document.pictureInPictureEnabled || allowed.length === 0) return { found: true, ok: false, reason: "disabled" };
  const area = (video) => {
    const box = video.getBoundingClientRect();
    return box.width * box.height;
  };
  const video = allowed.sort((a, b) => area(b) - area(a))[0];
  try {
    await video.requestPictureInPicture();
    return { found: true, ok: true };
  } catch (error) {
    // Chromium wants a user gesture: the toolbar button or the shortcut gives one.
    return { found: true, ok: false, reason: error && error.name === "NotAllowedError" ? "gesture" : "failed" };
  }
}

/** One answer from the frames of a tab: a success anywhere, else the frame with a video. */
function merge(results) {
  const answers = results.map((result) => result && result.result).filter(Boolean);
  if (answers.some((answer) => answer.ok && answer.found)) return { ok: true };
  const found = answers.find((answer) => answer.found);
  const reason = found ? found.reason : "noVideo";
  return { ok: false, reason: REASONS.includes(reason) ? reason : "failed" };
}

/** The tab to act on: the one floating, else one with a video, else an audible one. */
async function targetTab() {
  const known = [...frames.entries()];
  const pick = known.find(([, state]) => state.pip) ?? known.find(([, state]) => state.video);
  if (pick) return Number(pick[0].split(":")[0]);
  const tabs = await chrome.tabs.query({ url: PATTERN });
  const tab = tabs.find((t) => t.audible) ?? tabs[0];
  return tab ? tab.id : null;
}

async function run(action) {
  const tabId = await targetTab();
  if (tabId === null) return { ok: false, reason: "noVideo" };
  try {
    const results = await chrome.scripting.executeScript({ target: { tabId, allFrames: true }, func: floatVideo, args: [action] });
    return merge(results);
  } catch {
    return { ok: false, reason: "failed" };
  }
}

function aggregate() {
  const states = [...frames.values()];
  return { video: states.some((s) => s.video), pip: states.some((s) => s.pip) };
}

/** Tells îlot when the Crunchyroll tabs change. */
function publish() {
  if (!port) return;
  const state = aggregate();
  const text = JSON.stringify(state);
  if (text === sent) return;
  sent = text;
  port.postMessage({ type: "state", ...state });
}

async function onIlot(message) {
  if (!message || (message.type !== "pip" && message.type !== "exitPip")) return;
  const answer = await run(message.type === "pip" ? "enter" : "exit");
  if (message.type === "pip" && port) {
    port.postMessage(answer.ok ? { type: "pip", ok: true } : { type: "pip", ok: false, reason: answer.reason });
  }
}

/** Opens the link with îlot while a Crunchyroll tab is open (îlot starts the relay). */
async function connect() {
  const tabs = await chrome.tabs.query({ url: PATTERN });
  if (tabs.length === 0) {
    await chrome.alarms.clear(ALARM);
    return;
  }
  await chrome.alarms.create(ALARM, { periodInMinutes: RETRY_MINUTES });
  if (port) return;
  port = chrome.runtime.connectNative(HOST);
  sent = "";
  port.onMessage.addListener((message) => void onIlot(message));
  port.onDisconnect.addListener(() => {
    // îlot is closed, or not set up yet (LISEZMOI.md): the alarm tries again. Reading the
    // error tells the browser it was expected.
    port = null;
    return chrome.runtime.lastError;
  });
  port.postMessage({ type: "hello", version: PROTOCOL_VERSION });
  publish();
  // The worker may have slept: the pages say again what they hold.
  for (const tab of tabs) chrome.tabs.sendMessage(tab.id, { type: "report" }).catch(() => {});
}

chrome.runtime.onMessage.addListener((message, sender) => {
  if (sender.id !== chrome.runtime.id || !sender.tab || !message || message.type !== "state") return;
  frames.set(`${sender.tab.id}:${sender.frameId}`, { video: message.video === true, pip: message.pip === true });
  publish();
  void connect();
});

chrome.tabs.onRemoved.addListener((tabId) => {
  for (const key of [...frames.keys()]) if (key.startsWith(`${tabId}:`)) frames.delete(key);
  publish();
});

// A click on the button or the shortcut (global: it works even when the browser is in
// the background) is a user gesture: the browser accepts to float the video then.
chrome.action.onClicked.addListener(() => void run("toggle"));
chrome.commands.onCommand.addListener((command) => {
  if (command === "detacher") void run("toggle");
});

chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === ALARM) void connect();
});
chrome.runtime.onStartup.addListener(() => void connect());
chrome.runtime.onInstalled.addListener(() => void connect());
