/**
 * The seam between the web app and the desktop shell.
 *
 * Every function works in both environments: inside Tauri it calls the native
 * API, in a browser it uses the closest web platform equivalent. The Tauri
 * modules are imported dynamically so a plain browser never evaluates them.
 */

/** True when running inside the Tauri desktop shell. */
export const isDesktop: boolean =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface TimeSync {
  /** Milliseconds to add to the system clock to obtain network time. */
  offsetMs: number;
  /** Round-trip delay of the measurement in milliseconds. */
  roundTripMs: number;
  /** Host name of the server that answered. */
  server: string;
  /** Distance of the server from a reference clock (1 is best). */
  stratum: number;
}

/** Measures the system clock error against a network time server. */
export async function syncNetworkTime(): Promise<TimeSync> {
  if (!isDesktop) throw new Error("Network time is only available in the desktop app.");
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<TimeSync>("sync_time");
}

export async function setAlwaysOnTop(enabled: boolean): Promise<void> {
  if (!isDesktop) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().setAlwaysOnTop(enabled);
}

/** Toggles fullscreen and resolves to the new state. */
export async function toggleFullscreen(): Promise<boolean> {
  if (isDesktop) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const next = !(await win.isFullscreen());
    await win.setFullscreen(next);
    return next;
  }
  if (document.fullscreenElement) {
    await document.exitFullscreen();
    return false;
  }
  await document.documentElement.requestFullscreen();
  return true;
}

/** Asks for permission to show notifications. Resolves to whether it is granted. */
export async function requestNotificationPermission(): Promise<boolean> {
  if (isDesktop) {
    const n = await import("@tauri-apps/plugin-notification");
    if (await n.isPermissionGranted()) return true;
    return (await n.requestPermission()) === "granted";
  }
  if (!("Notification" in window)) return false;
  if (Notification.permission === "granted") return true;
  if (Notification.permission === "denied") return false;
  return (await Notification.requestPermission()) === "granted";
}

/** Shows a system notification. Resolves to false when it could not be shown. */
export async function notify(title: string, body: string): Promise<boolean> {
  try {
    if (isDesktop) {
      const n = await import("@tauri-apps/plugin-notification");
      if (!(await n.isPermissionGranted())) return false;
      n.sendNotification({ title, body });
      return true;
    }
    if (!("Notification" in window) || Notification.permission !== "granted") return false;
    new Notification(title, { body });
    return true;
  } catch {
    return false;
  }
}

/**
 * Saves a calendar file. On the desktop it is written to the user's download
 * folder and the full path is returned; in a browser it is offered as a
 * download and `null` is returned.
 */
export async function saveCalendar(fileName: string, contents: string): Promise<string | null> {
  if (isDesktop) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("save_calendar", { fileName, contents });
  }
  const url = URL.createObjectURL(new Blob([contents], { type: "text/calendar;charset=utf-8" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return null;
}

/** Copies text to the clipboard. Resolves to whether it succeeded. */
export async function copyText(text: string): Promise<boolean> {
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
