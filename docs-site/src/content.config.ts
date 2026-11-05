import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro/zod";
import { docGroups } from "./lib/docs";

const docs = defineCollection({
  loader: glob({ pattern: "**/*.md", base: "./src/content/docs" }),
  schema: z.object({
    title: z.string().min(1),
    description: z.string().min(1).max(160),
    group: z.enum(docGroups),
    order: z.number().int(),
    sidebarLabel: z.string().min(1).optional(),
    featured: z.boolean().optional(),
  }),
});

const blog = defineCollection({
  loader: glob({ pattern: "**/*.mdx", base: "./src/content/blog" }),
  schema: z
    .object({
      title: z.string().min(1).max(48),
      headline: z.string().min(1).max(90).optional(),
      description: z.string().min(120).max(160),
      published: z.coerce.date(),
      updated: z.coerce.date().optional(),
      keyword: z.string().min(1),
      feature: z.enum([
        "disk",
        "remote",
        "gpu",
        "menu-bar",
        "network",
        "memory",
        "cli",
        "power",
        "battery",
        "timeline",
        "containers",
        "processes",
        "cpu",
      ]),
      relatedDocs: z.array(z.string()),
    })
    .refine((post) => !post.updated || post.updated >= post.published, {
      message: "updated must be on or after published",
      path: ["updated"],
    }),
});

export const collections = { docs, blog };
