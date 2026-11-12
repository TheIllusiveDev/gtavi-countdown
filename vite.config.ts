import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// TAURI_DEV_HOST is set by the Tauri CLI when developing against a physical
// device; on the desktop it is undefined and the server binds to localhost.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  // Relative asset URLs keep the bundle portable: it works from any sub-path
  // on a static host and from the tauri:// custom protocol alike.
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    // The embedded webviews (WebView2, WKWebView, WebKitGTK) and every current
    // browser support this baseline, so no legacy transforms are emitted.
    target: "es2022",
    cssMinify: true,
    reportCompressedSize: true,
  },
});
