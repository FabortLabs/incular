import { readdirSync } from "node:fs";
import tailwindcss from "@tailwindcss/vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import react from "@vitejs/plugin-react";
import { fumadocsMdx } from "fumadocs-mdx/vite";
import { nitro } from "nitro/vite";
import { defineConfig } from "vite";

const base = "/incular/";
const docPaths = readdirSync("content/docs", { recursive: true })
  .map(String)
  .filter((path) => path.endsWith(".mdx"))
  .map((path) => path.replaceAll("\\", "/").replace(/\.mdx$/, ""))
  .map((path) => path.replace(/(^|\/)index$/, "").replace(/\/$/, ""));

export default defineConfig({
  base,
  server: {
    port: 3535,
  },
  plugins: [
    fumadocsMdx(),
    tailwindcss(),
    tanstackStart({
      router: { basepath: base },
    }),
    react(),
    nitro({
      preset: "github_pages",
      baseURL: base,
      prerender: {
        routes: [
          base,
          ...docPaths.flatMap((path) => [
            `${base}docs/${path}`,
            `${base}docs/${path || "index"}.md`,
          ]),
          `${base}api/docs.json`,
          `${base}api/search.json`,
          `${base}llms.txt`,
          `${base}llms-full.txt`,
        ],
        failOnError: true,
      },
    }),
  ],
  resolve: {
    tsconfigPaths: true,
    alias: {
      tslib: "tslib/tslib.es6.js",
    },
  },
});
