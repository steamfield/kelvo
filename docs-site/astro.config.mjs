// @ts-check
import { fileURLToPath } from "node:url";
import { defineConfig } from "astro/config";
import { satteri } from "@astrojs/markdown-satteri";
import react from "@astrojs/react";
import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
import { createHastPlugins, createShikiConfig } from "@steamfield/ui/markdown";

const SITE_URL = "https://statglass.steamfield.io";

// docs-site renders real StatGlass widgets. Its own code may only import the
// two `@statglass/*` entry points; `~` and `@core` exist because the widgets
// import through them, and must match the root vite.config.ts aliases.
/** @param {string} path */
const app = (path) => fileURLToPath(new URL(`../src/${path}`, import.meta.url));
export const appAlias = {
  "@statglass/widgets": app("app/widgets/index.ts"),
  "@statglass/tokens.css": app("app/styles/tokens.css"),
  "~": app("app"),
  "@core": app("core"),
};

// https://astro.build/config
export default defineConfig({
  site: SITE_URL,
  output: "static",
  trailingSlash: "never",
  build: { format: "file" },
  integrations: [react(), sitemap()],
  // Docs markdown: fence titles and groups, callouts, heading links
  // (@steamfield/ui/markdown). Smart punctuation stays off so --flags stay intact.
  markdown: {
    shikiConfig: createShikiConfig(),
    processor: satteri({
      hastPlugins: createHastPlugins(),
      features: { smartPunctuation: false },
    }),
  },
  vite: {
    plugins: [tailwindcss()],
    resolve: {
      alias: appAlias,
      // Widgets resolve react from the root node_modules; two copies break hooks.
      dedupe: ["react", "react-dom"],
    },
    server: {
      fs: {
        allow: [
          fileURLToPath(new URL(".", import.meta.url)),
          app(""),
          fileURLToPath(new URL("../node_modules", import.meta.url)),
        ],
      },
    },
  },
});
