import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Relative asset URLs keep the bundle portable: it works from any sub-path
  // on a static host.
  base: "./",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  css: {
    modules: {
      // Readable class names in development, short ones in production.
      generateScopedName: process.env.NODE_ENV === "production" ? "[hash:base64:6]" : "[name]__[local]",
    },
  },
  build: {
    // Every current browser supports this baseline, so no legacy transforms
    // are emitted.
    target: "es2022",
    cssMinify: true,
    reportCompressedSize: true,
  },
});
