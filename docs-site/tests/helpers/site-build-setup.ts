import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { TestProject } from "vitest/node";
import { buildSite } from "./frontend-build";

declare module "vitest" {
  export interface ProvidedContext {
    /** The production build every build test reads (`inject("siteDir")`). */
    siteDir: string;
  }
}

/**
 * Builds the site once per run. Two `astro build`s at the same time share
 * `.astro/.prerender` and break each other, so test files must not build in
 * parallel; a build that needs other env (a PostHog key) runs inside its test.
 */
export default async function setup(project: TestProject) {
  const root = mkdtempSync(join(tmpdir(), "statglass-site-"));
  const siteDir = join(root, "site");
  const build = await buildSite(siteDir);
  if (build.status !== 0) {
    rmSync(root, { recursive: true, force: true });
    throw new Error(`astro build failed:\n${build.stderr}`);
  }
  project.provide("siteDir", siteDir);
  return () => rmSync(root, { recursive: true, force: true });
}
