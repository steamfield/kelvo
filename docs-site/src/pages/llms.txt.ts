import { getCollection } from "astro:content";
import type { APIRoute } from "astro";
import { blogPath, sortedPosts } from "../lib/blog";
import { llmsTxt } from "../lib/docs";

// The llmstxt.org index of the docs, prerendered at build time.
export const GET: APIRoute = async ({ site }) =>
  new Response(
    llmsTxt(await getCollection("docs"), String(site)) +
      "\n## Blog\n\n" +
      sortedPosts(await getCollection("blog"))
        .map(
          (post) =>
            `- [${post.data.title}](${new URL(blogPath(post.id), site).href}): ${post.data.description}`
        )
        .join("\n") +
      "\n",
    {
      headers: { "Content-Type": "text/plain; charset=utf-8" },
    }
  );
