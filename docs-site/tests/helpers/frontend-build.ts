import { execFile } from "node:child_process";
import { fileURLToPath } from "node:url";

export interface SiteBuild {
  status: number;
  stdout: string;
  stderr: string;
}

/** Runs a production `astro build` into outDir (the demo comes from public/). */
export function buildSite(outDir: string): Promise<SiteBuild> {
  const astro = fileURLToPath(
    new URL("../../node_modules/.bin/astro", import.meta.url)
  );
  return new Promise((resolve, reject) => {
    execFile(
      astro,
      ["build", "--outDir", outDir],
      {
        cwd: fileURLToPath(new URL("../../", import.meta.url)),
        env: {
          PATH: process.env.PATH,
          ASTRO_TELEMETRY_DISABLED: "1",
          NODE_ENV: "production",
        },
        timeout: 120_000,
        maxBuffer: 8 * 1024 * 1024,
      },
      (error, stdout, stderr) => {
        if (!error) resolve({ status: 0, stdout, stderr });
        else if (typeof error.code === "number")
          resolve({ status: error.code, stdout, stderr });
        else reject(error);
      }
    );
  });
}
