import { useSyncExternalStore } from "react";

const query = typeof matchMedia === "function" ? matchMedia("(prefers-reduced-motion: reduce)") : null;

function subscribe(listener: () => void): () => void {
  query?.addEventListener("change", listener);
  return () => query?.removeEventListener("change", listener);
}

/** Whether the operating system asks for reduced motion, kept up to date. */
export function useReducedMotion(): boolean {
  return useSyncExternalStore(subscribe, () => query?.matches ?? false);
}
