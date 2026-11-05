import rss from "@astrojs/rss";
import { getCollection } from "astro:content";
import type { APIRoute } from "astro";
import { blogDescription, blogPath, sortedPosts } from "@/lib/blog";

export const GET: APIRoute = async ({ site }) =>
  rss({
    title: "StatGlass blog",
    description: blogDescription,
    site: String(site),
    trailingSlash: false,
    items: sortedPosts(await getCollection("blog")).map((entry) => ({
      title: entry.data.title,
      description: entry.data.description,
      pubDate: entry.data.published,
      link: blogPath(entry.id),
    })),
  });
