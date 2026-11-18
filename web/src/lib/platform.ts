/**
 * Browser capabilities, each wrapped so that the app degrades gracefully
 * where one is missing. The native desktop version is the Rust app in
 * `rust/`; this module has no desktop branch.
 */

/**
 * True on phones and tablets: no hover and a coarse pointer. Used to swap
 * desktop affordances (copy, fullscreen, shortcuts) for touch ones.
 */
export const isTouch: boolean =
  typeof matchMedia === "function" && matchMedia("(hover: none) and (pointer: coarse)").matches;

/** False where the page cannot go fullscreen, notably Safari on the iPhone. */
export const canFullscreen: boolean =
  typeof document !== "undefined" && document.fullscreenEnabled === true;

/** True where the system share sheet is available and preferable to copying. */
export const canShare: boolean =
  isTouch && typeof navigator !== "undefined" && typeof navigator.share === "function";

/**
 * Opens the system share sheet. Resolves to "shared", "cancelled" when the
 * user dismissed it, or "failed".
 */
export async function shareText(
  title: string,
  text: string,
): Promise<"shared" | "cancelled" | "failed"> {
  try {
    await navigator.share({ title, text, url: location.href });
    return "shared";
  } catch (error) {
    return error instanceof DOMException && error.name === "AbortError" ? "cancelled" : "failed";
  }
}

/** Toggles fullscreen and resolves to the new state. */
export async function toggleFullscreen(): Promise<boolean> {
  if (document.fullscreenElement) {
    await document.exitFullscreen();
    return false;
  }
  await document.documentElement.requestFullscreen();
  return true;
}

/** Asks for permission to show notifications. Resolves to whether it is granted. */
export async function requestNotificationPermission(): Promise<boolean> {
  if (!("Notification" in window)) return false;
  if (Notification.permission === "granted") return true;
  if (Notification.permission === "denied") return false;
  return (await Notification.requestPermission()) === "granted";
}

/** Shows a system notification. Resolves to false when it could not be shown. */
export async function notify(title: string, body: string): Promise<boolean> {
  try {
    if (!("Notification" in window) || Notification.permission !== "granted") return false;
    new Notification(title, { body });
    return true;
  } catch {
    return false;
  }
}

/** Offers a calendar file as a download. */
export function saveCalendar(fileName: string, contents: string): void {
  const url = URL.createObjectURL(new Blob([contents], { type: "text/calendar;charset=utf-8" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
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
