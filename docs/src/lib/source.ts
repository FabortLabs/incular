import { loader } from "fumadocs-core/source";
import { lucideIconsPlugin } from "fumadocs-core/source/lucide-icons";
import { docs } from "./content";
import { docsRoute, getPageMarkdownUrl } from "./shared";

export const source = loader({
  source: docs.toFumadocsSource(),
  baseUrl: docsRoute,
  plugins: [lucideIconsPlugin()],
});

export async function getDocsData() {
  return {
    pages: source.getPages().map((page) => ({
      slugs: page.slugs,
      path: page.path,
      markdownUrl: getPageMarkdownUrl(page).url,
    })),
    pageTree: await source.serializePageTree(source.getPageTree()),
  };
}
