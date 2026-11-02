import { getCollection } from "astro:content";
import type { APIRoute } from "astro";
import { llmsTxt } from "../lib/docs";

// The llmstxt.org index of the docs, prerendered at build time.
export const GET: APIRoute = async ({ site }) =>
  new Response(llmsTxt(await getCollection("docs"), String(site)), {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  });
