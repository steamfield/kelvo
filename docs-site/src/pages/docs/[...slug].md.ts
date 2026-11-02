import { getCollection } from "astro:content";
import type { APIRoute, GetStaticPaths } from "astro";
import { markdownExport } from "../../lib/docs";

// Prerendered at build time: each docs page also ships as /docs/<id>.md for agents.
export const getStaticPaths = (async () => {
  const entries = await getCollection("docs");
  return entries.map((entry) => ({
    params: { slug: entry.id },
    props: { entry },
  }));
}) satisfies GetStaticPaths;

export const GET: APIRoute = async ({ props, site }) => {
  const entries = await getCollection("docs");
  return new Response(markdownExport(props.entry, entries, String(site)), {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  });
};
