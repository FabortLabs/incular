import { llms } from "fumadocs-core/source";
import { source } from "./source";

const base = import.meta.env.BASE_URL.replace(/\/$/, "");

export const docsLlms = llms(source, {
  renderPage: async (page) => `# ${page.data.title} (${base}${page.url})

${(await page.data.getText("processed")).replaceAll("](/docs", `](${base}/docs`)}`,
});
