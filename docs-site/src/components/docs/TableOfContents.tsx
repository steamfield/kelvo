export type DocHeading = { depth: number; slug: string; text: string };

/** "On this page" for a page with at least three sections. docs-enhance marks the section in view. */
export function TableOfContents({
  headings,
}: {
  headings: readonly DocHeading[];
}) {
  const sections = headings.filter(
    (heading) => heading.depth === 2 || heading.depth === 3
  );
  if (sections.length < 3) return null;
  return (
    <nav aria-label="On this page" data-toc className="sticky top-24 text-xs">
      <p className="mb-3 text-[11px] text-muted-foreground">On this page</p>
      <ul className="space-y-1 border-l border-border">
        {sections.map((heading) => (
          <li key={heading.slug}>
            <a
              href={`#${heading.slug}`}
              data-toc-link
              className={`-ml-px block border-l border-transparent py-0.5 text-muted-foreground hover:text-foreground aria-[current=location]:border-link aria-[current=location]:text-foreground ${heading.depth === 3 ? "pl-6" : "pl-3"}`}
            >
              {heading.text}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}
