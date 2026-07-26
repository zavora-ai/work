import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The renderer is a plain bundle with no Node access. Its only capability is the
// preload bridge, which the main process injects.
export default defineConfig({
  root: "src/renderer",
  base: "./",
  plugins: [react()],
  build: {
    outDir: "../../dist/renderer",
    emptyOutDir: true,
    target: "es2023",
  },
});
