import { createFileRoute, notFound } from "@tanstack/react-router";
import { createIsomorphicFn } from "@tanstack/react-start";
import { useFumadocsLoader } from "fumadocs-core/source/client";
import { DocsLayout } from "fumadocs-ui/layouts/docs";
import {
  DocsBody,
  DocsDescription,
  DocsPage,
  DocsTitle,
  MarkdownCopyButton,
  ViewOptionsPopover,
} from "fumadocs-ui/layouts/docs/page";
import { Suspense, use } from "react";
import { useMDXComponents } from "@/components/mdx";
import { docs } from "@/lib/content";
import { baseOptions } from "@/lib/layout.shared";
import { gitConfig } from "@/lib/shared";
import { getDocsData } from "@/lib/source";

let staticData: Promise<Awaited<ReturnType<typeof getDocsData>>> | undefined;
const loadDocsData = createIsomorphicFn()
  .server(getDocsData)
  .client(() => {
    staticData ??= fetch(`${import.meta.env.BASE_URL}api/docs.json`).then(
      async (response) => {
        if (!response.ok) throw new Error("Unable to load documentation index");
        return response.json();
      },
    );
    return staticData;
  });

export const Route = createFileRoute("/docs/$")({
  component: Page,
  loader: async ({ params }) => {
    const slugs = params._splat?.split("/").filter(Boolean) ?? [];
    const data = await loadDocsData();
    const page = data.pages.find(
      (page) => page.slugs.join("/") === slugs.join("/"),
    );
    if (!page) throw notFound();
    await docs.getPage(page.path)?.preload();
    return {
      path: page.path,
      markdownUrl: page.markdownUrl,
      pageTree: data.pageTree,
    };
  },
});

function Content({ path, markdownUrl }: { path: string; markdownUrl: string }) {
  const page = docs.getPage(path);
  if (!page) throw new Error(`unknown page: ${path}`);

  const { toc } = use(page.load());
  const MDX = page.body;

  return (
    <DocsPage toc={toc}>
      <DocsTitle>{page.title}</DocsTitle>
      <DocsDescription>{page.description}</DocsDescription>
      <div className="flex flex-row gap-2 items-center border-b -mt-4 pb-6">
        <MarkdownCopyButton markdownUrl={markdownUrl} />
        <ViewOptionsPopover
          markdownUrl={markdownUrl}
          githubUrl={`https://github.com/${gitConfig.user}/${gitConfig.repo}/blob/${gitConfig.branch}/docs/content/docs/${path}`}
        />
      </div>
      <DocsBody>
        <MDX components={useMDXComponents()} />
      </DocsBody>
    </DocsPage>
  );
}

function Page() {
  const { path, markdownUrl, pageTree } = useFumadocsLoader(
    Route.useLoaderData(),
  );

  return (
    <DocsLayout {...baseOptions()} tree={pageTree}>
      <Suspense>
        <Content path={path} markdownUrl={markdownUrl} />
      </Suspense>
    </DocsLayout>
  );
}
