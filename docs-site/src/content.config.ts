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

export const collections = { docs };
