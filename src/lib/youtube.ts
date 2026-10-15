// YouTube links pasted in the mini-player. Only the video id leaves this file: the Rust
// core checks it again and builds the official embed URL itself (src-tauri/src/players.rs).

const VIDEO_ID = /^[A-Za-z0-9_-]{11}$/;
const HOSTS = ["youtube.com", "youtube-nocookie.com", "youtu.be"];
/** Path prefixes followed by the video id. */
const ID_PATHS = ["/shorts/", "/embed/", "/live/", "/v/"];

function parseUrl(input: string): URL | null {
  const text = /^[a-z]+:\/\//i.test(input) ? input : `https://${input}`;
  try {
    return new URL(text);
  } catch {
    return null;
  }
}

function youtubeUrl(input: string): URL | null {
  const url = parseUrl(input.trim());
  if (!url || (url.protocol !== "https:" && url.protocol !== "http:")) return null;
  const host = url.hostname.toLowerCase().replace(/^(www|m|music)\./, "");
  return HOSTS.includes(host) ? url : null;
}

/** Video id of a YouTube link (watch, youtu.be, shorts, embed, live), or of a bare id. */
export function youtubeVideoId(input: string): string | null {
  const text = input.trim();
  if (VIDEO_ID.test(text)) return text;
  const url = youtubeUrl(text);
  if (!url) return null;
  let id: string | null = null;
  if (url.hostname.endsWith("youtu.be")) {
    id = url.pathname.split("/")[1] ?? null;
  } else if (url.pathname === "/watch") {
    id = url.searchParams.get("v");
  } else {
    const prefix = ID_PATHS.find((p) => url.pathname.startsWith(p));
    if (prefix) id = url.pathname.slice(prefix.length).split("/")[0];
  }
  return id && VIDEO_ID.test(id) ? id : null;
}

/** Start time of a link in seconds: `t=90`, `t=1m30s`, `t=1h2m3s` or `start=90`. */
export function youtubeStart(input: string): number {
  const url = youtubeUrl(input);
  const value = url?.searchParams.get("t") ?? url?.searchParams.get("start");
  if (!value) return 0;
  if (/^\d+s?$/.test(value)) return parseInt(value, 10);
  const parts = /^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?$/.exec(value);
  if (!parts || !parts[0]) return 0;
  const [, h = "0", m = "0", s = "0"] = parts;
  return Number(h) * 3600 + Number(m) * 60 + Number(s);
}
