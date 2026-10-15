/* global chrome, document, addEventListener */
// Runs in the Crunchyroll pages and in their player frame (extension/manifest.json). It
// only tells the extension whether a video is here and whether the browser shows it
// floating: two yes-or-no, nothing about the episode, the address or the picture.

(() => {
  const EVENTS = ["loadedmetadata", "emptied", "enterpictureinpicture", "leavepictureinpicture"];
  let last = "";

  function report(force = false) {
    const video = [...document.querySelectorAll("video")].some((element) => element.readyState > 0);
    const pip = document.pictureInPictureElement !== null;
    const now = `${video}:${pip}`;
    if (now === last && !force) return;
    last = now;
    chrome.runtime.sendMessage({ type: "state", video, pip }).catch(() => {});
  }

  // These media events do not bubble: they are caught on their way down.
  for (const name of EVENTS) document.addEventListener(name, () => report(), true);
  addEventListener("pagehide", () => {
    last = "false:false";
    chrome.runtime.sendMessage({ type: "state", video: false, pip: false }).catch(() => {});
  });
  chrome.runtime.onMessage.addListener((message, sender) => {
    if (sender.id === chrome.runtime.id && message && message.type === "report") report(true);
  });
  report();
})();
