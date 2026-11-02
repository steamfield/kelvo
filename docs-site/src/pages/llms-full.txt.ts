import { getCollection } from "astro:content";
import type { APIRoute } from "astro";
import { llmsFull } from "../lib/docs";

// Every docs page in one file, prerendered at build time.
export const GET: APIRoute = async ({ site }) =>
  new Response(llmsFull(await getCollection("docs"), String(site)), {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  });
