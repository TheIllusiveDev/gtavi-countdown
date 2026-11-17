/**
 * The browser side of the Blazor UI.
 *
 * Everything that has to run per frame (the backdrop, the milliseconds
 * readout, the layout measurements the backdrop is composed around) lives
 * here, so that .NET is only involved once per second. The rest are thin
 * wrappers over web platform APIs that Blazor cannot reach directly.
 */

import { Scene } from "./scene.js";

// ------------------------------------------------------------- environment

const matches = (query) => typeof matchMedia === "function" && matchMedia(query).matches;

/** What this browser can do; read once at start-up. */
export function capabilities() {
  return {
    // Phones and tablets: no hover and a coarse pointer.
    isTouch: matches("(hover: none) and (pointer: coarse)"),
    // False notably in Safari on the iPhone.
    fullscreenEnabled: document.fullscreenEnabled === true,
    share: typeof navigator.share === "function",
    notifications: "Notification" in window,
  };
}

/** Routes window events the .NET side cares about to `receiver`. */
export function attachWindow(receiver) {
  const resync = () => receiver.invokeMethodAsync("OnResync");
  const fullscreen = () => receiver.invokeMethodAsync("OnFullscreenChanged", document.fullscreenElement !== null);
  const keydown = (event) => {
    if (event.ctrlKey || event.metaKey || event.altKey || event.repeat) return;
    if (event.target instanceof Element && event.target.closest("input, textarea, select, [contenteditable]")) return;
    const key = event.key.toLowerCase();
    if (!["s", "f", "m", "t", "c"].includes(key)) return;
    // Decided here, synchronously, because the call into .NET is asynchronous.
    event.preventDefault();
    receiver.invokeMethodAsync("OnShortcut", key);
  };

  document.addEventListener("visibilitychange", resync);
  window.addEventListener("focus", resync);
  window.addEventListener("pageshow", resync);
  document.addEventListener("fullscreenchange", fullscreen);
  window.addEventListener("keydown", keydown);
  return {
    dispose() {
      document.removeEventListener("visibilitychange", resync);
      window.removeEventListener("focus", resync);
      window.removeEventListener("pageshow", resync);
      document.removeEventListener("fullscreenchange", fullscreen);
      window.removeEventListener("keydown", keydown);
    },
  };
}

// ---------------------------------------------------------------- document

export function setTheme(theme) {
  document.documentElement.dataset.theme = theme;
  const base = getComputedStyle(document.documentElement).getPropertyValue("--base").trim();
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", base);
}

export function setTitle(title) {
  document.title = title;
}

// ----------------------------------------------------------------- storage

export function storageGet(key) {
  try {
    return localStorage.getItem(key);
  } catch {
    // Storage can be unavailable (private windows, blocked site data).
    return null;
  }
}

export function storageSet(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Persistence is a convenience; the app works without it.
  }
}

// ----------------------------------------------------------------- actions

/** Copies text to the clipboard. Resolves to whether it succeeded. */
export async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Fallback for contexts without the asynchronous clipboard API.
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.append(area);
    area.select();
    let ok = false;
    try {
      ok = document.execCommand("copy");
    } catch {
      ok = false;
    }
    area.remove();
    return ok;
  }
}

/** Opens the system share sheet. Resolves to "shared", "cancelled" or "failed". */
export async function shareText(title, text) {
  try {
    await navigator.share({ title, text, url: location.href });
    return "shared";
  } catch (error) {
    return error instanceof DOMException && error.name === "AbortError" ? "cancelled" : "failed";
  }
}

/** Toggles fullscreen and resolves to the new state. */
export async function toggleFullscreen() {
  if (document.fullscreenElement) {
    await document.exitFullscreen();
    return false;
  }
  await document.documentElement.requestFullscreen();
  return true;
}

/** Asks for permission to show notifications. Resolves to whether it is granted. */
export async function requestNotificationPermission() {
  if (!("Notification" in window)) return false;
  if (Notification.permission === "granted") return true;
  if (Notification.permission === "denied") return false;
  return (await Notification.requestPermission()) === "granted";
}

/** Shows a system notification. Returns false when it could not be shown. */
export function notify(title, body) {
  try {
    if (!("Notification" in window) || Notification.permission !== "granted") return false;
    new Notification(title, { body });
    return true;
  } catch {
    return false;
  }
}

/** Offers text as a file download. */
export function download(fileName, contents, type) {
  const url = URL.createObjectURL(new Blob([contents], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

// ------------------------------------------------------------------ dialog

/**
 * Wires a native dialog to .NET. The dialog provides the focus trap, Escape
 * handling and inert background; .NET hears about every way it can close.
 */
export function initDialog(dialog, receiver) {
  dialog.addEventListener("close", () => receiver.invokeMethodAsync("OnDialogClosed"));
  dialog.addEventListener("click", (event) => {
    // A click on the backdrop lands on the dialog element itself.
    if (event.target === dialog) dialog.close();
  });
}

export function setDialogOpen(dialog, open) {
  if (open && !dialog.open) dialog.showModal();
  else if (!open && dialog.open) dialog.close();
}

// ---------------------------------------------------------- milliseconds

/**
 * Writes the milliseconds readout from an animation frame. Routing it
 * through .NET would re-render sixty times a second, so the frame loop
 * writes the text node directly and nothing else is touched.
 */
export function startMillis(node, target, offset) {
  let goal = target;
  let correction = offset;
  let last = "";
  let frame = 0;
  const paint = () => {
    frame = requestAnimationFrame(paint);
    const text = String(Math.floor(Math.abs(goal - (Date.now() + correction)) % 1000)).padStart(3, "0");
    if (text !== last) {
      node.textContent = text;
      last = text;
    }
  };
  paint();
  return {
    update(nextTarget, nextOffset) {
      goal = nextTarget;
      correction = nextOffset;
    },
    stop() {
      cancelAnimationFrame(frame);
    },
  };
}

// ---------------------------------------------------------------- backdrop

const motionQuery =
  typeof matchMedia === "function" ? matchMedia("(prefers-reduced-motion: reduce)") : null;

let scene = null;
let composition = { horizon: 0.66, sunRadius: 0.16 };

function applyMotion() {
  scene?.setAnimated(!(motionQuery?.matches ?? false));
}
motionQuery?.addEventListener("change", applyMotion);

/**
 * Creates the renderer on `canvas`. Returns false when WebGL is unavailable,
 * in which case the CSS gradient behind the canvas stays visible.
 */
export function mountBackdrop(canvas, palette, quality) {
  unmountBackdrop();
  const created = new Scene(canvas, palette);
  if (!created.init()) return false;
  scene = created;
  scene.setComposition(composition);
  scene.setQuality(quality);
  applyMotion();
  return true;
}

export function updateBackdrop(palette, quality) {
  if (!scene) return;
  scene.setPalette(palette);
  scene.setQuality(quality);
}

/** Tears the renderer down so that no GPU memory or context is held needlessly. */
export function unmountBackdrop() {
  scene?.destroy();
  scene = null;
}

/**
 * Composes the backdrop around the content: the horizon sits just above the
 * lower panels and the sun fills the gap beneath the digits, whatever the
 * viewport. Positions are measured in document space, so scrolling a tall
 * page does not move the scene.
 */
export function observeComposition(hero, details) {
  const root = document.documentElement;
  const measure = () => {
    const height = window.innerHeight || 1;
    const heroBottom = hero.offsetTop + hero.offsetHeight;
    // In the compact layout the lower panels are hidden; the horizon then
    // drops to the bottom of the window, clear of the timer.
    const hidden = details.offsetParent === null;
    const anchor = hidden ? Math.max(heroBottom + 44, height * 0.82) : details.offsetTop - 18;
    const horizon = Math.min(Math.max(anchor, height * 0.42), height * 0.9);
    // The sun is sunk 12% below the horizon, so 1.12 radii show above it.
    const room = (horizon - heroBottom - 6) / 1.12;
    const radius = Math.min(Math.max(room, 30), height * 0.2);
    const next = { horizon: horizon / height, sunRadius: radius / height };
    if (
      Math.abs(next.horizon - composition.horizon) > 0.001 ||
      Math.abs(next.sunRadius - composition.sunRadius) > 0.001
    ) {
      composition = next;
      // The fallback gradient reads the horizon from this property.
      root.style.setProperty("--horizon", `${(next.horizon * 100).toFixed(2)}%`);
      scene?.setComposition(next);
    }
  };
  const observer = new ResizeObserver(measure);
  observer.observe(hero);
  observer.observe(details);
  window.addEventListener("resize", measure);
  measure();
  return {
    dispose() {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    },
  };
}
