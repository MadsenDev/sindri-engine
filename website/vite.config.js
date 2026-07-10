import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  // Relative base so the built site works at any mount point,
  // including the GitHub Pages project path (/sindri-engine/).
  base: "./",
  plugins: [react()],
});
