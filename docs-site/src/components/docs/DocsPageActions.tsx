import { CopyButton } from "@/components/CopyButton";

const linkClass =
  "inline-flex items-center rounded-control border border-border bg-card px-2.5 py-1.5 text-xs font-[510] text-foreground no-underline hover:bg-secondary";

/**
 * Ways to take a page to an agent: copy its Markdown (BaseLayout's copy script reads the hidden
 * copy), open the Markdown file, or start a chat that reads it.
 */
export function DocsPageActions({
  markdown,
  markdownHref,
  markdownUrl,
}: {
  markdown: string;
  markdownHref: string;
  markdownUrl: string;
}) {
  const prompt = encodeURIComponent(
    `Read ${markdownUrl} and answer my questions about StatGlass.`
  );
  return (
    <div className="flex flex-wrap items-center gap-2">
      <pre id="docs-page-markdown" hidden>
        {markdown}
      </pre>
      <CopyButton id="docs-page-markdown" label="Copy page" />
      <a href={markdownHref} className={linkClass}>
        View as Markdown
      </a>
      <a
        href={`https://claude.ai/new?q=${prompt}`}
        target="_blank"
        rel="noopener noreferrer"
        className={linkClass}
      >
        Open in Claude
      </a>
      <a
        href={`https://chatgpt.com/?q=${prompt}`}
        target="_blank"
        rel="noopener noreferrer"
        className={linkClass}
      >
        Open in ChatGPT
      </a>
    </div>
  );
}
