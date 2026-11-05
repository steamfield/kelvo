import type { ReactNode } from "react";
import type { BlogEntry } from "@/lib/blog";
import { blogPath, readingMinutes } from "@/lib/blog";
import type { DocLink } from "@/lib/docs";
import prose from "@/components/docs/DocsProse.module.css";
import type { DocHeading } from "@/components/docs/TableOfContents";

export function BlogArticle({
  entry,
  headings,
  related,
  docs,
  children,
}: {
  entry: BlogEntry;
  headings: readonly DocHeading[];
  related: readonly BlogEntry[];
  docs: readonly DocLink[];
  children: ReactNode;
}) {
  const sections = headings.filter((h) => h.depth === 2);
  const date = (value: Date) =>
    value.toLocaleDateString("en-US", {
      timeZone: "UTC",
      year: "numeric",
      month: "long",
      day: "numeric",
    });
  return (
    // The container lets ArticleDemo break out to the page column's width.
    <div className="@container">
      <article className="mx-auto max-w-[47.5rem] min-w-0 pt-8 sm:pt-12">
        <header className="space-y-4 border-b border-border pb-6">
          <nav
            aria-label="Breadcrumb"
            className="text-xs text-muted-foreground"
          >
            <ol className="flex flex-wrap items-center gap-1.5">
              <li>
                <a href="/blog" className="hover:text-link">
                  Blog
                </a>
              </li>
              <li className="before:mr-1.5 before:content-['/']">
                {entry.data.title}
              </li>
            </ol>
          </nav>
          <h1 className="text-[clamp(2rem,4vw,3rem)] leading-tight font-[590] tracking-[-0.03em] text-balance">
            {entry.data.headline ?? entry.data.title}
          </h1>
          <p className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
            <time dateTime={entry.data.published.toISOString()}>
              {date(entry.data.published)}
            </time>
            {entry.data.updated && (
              <span>
                Updated{" "}
                <time dateTime={entry.data.updated.toISOString()}>
                  {date(entry.data.updated)}
                </time>
              </span>
            )}
            <span>{readingMinutes(entry.body ?? "")} min read</span>
          </p>
          {sections.length >= 4 && (
            <nav aria-label="On this page" className="text-xs">
              <p className="mb-2 text-muted-foreground">On this page</p>
              <ul className="flex flex-wrap gap-x-4 gap-y-2">
                {sections.map((h) => (
                  <li key={h.slug}>
                    <a
                      href={`#${h.slug}`}
                      className="text-muted-foreground hover:text-link"
                    >
                      {h.text}
                    </a>
                  </li>
                ))}
              </ul>
            </nav>
          )}
        </header>
        <div className={`${prose.prose} ${prose.article} mt-8`}>{children}</div>
        <section
          data-reveal=""
          className="flex flex-wrap items-center justify-between gap-6 border-t border-border py-16 sm:py-20"
        >
          <p
            data-motion-part=""
            className="max-w-xl text-xl leading-snug font-[590] tracking-[-0.01em] text-balance"
          >
            StatGlass is free under the Apache-2.0 license.
          </p>
          <a
            data-motion-part=""
            href="/docs/install"
            data-track-install="blog"
            className="inline-flex min-h-11 items-center rounded-control bg-primary px-5 text-sm font-[510] text-primary-foreground hover:opacity-90"
          >
            Install StatGlass
          </a>
        </section>
        {(related.length > 0 || docs.length > 0) && (
          <section aria-labelledby="related">
            <h2 id="related" className="mb-4 text-xl font-[590] tracking-tight">
              Related
            </h2>
            <ul className="divide-y divide-border border-y border-border">
              {related.map((post) => (
                <li key={post.id} className="py-3">
                  <a
                    href={blogPath(post.id)}
                    className="text-sm text-link hover:underline"
                  >
                    {post.data.title}
                  </a>
                </li>
              ))}
              {docs.map((doc) => (
                <li key={doc.href} className="py-3">
                  <a
                    href={doc.href}
                    className="text-sm text-link hover:underline"
                  >
                    {doc.label}
                  </a>
                </li>
              ))}
            </ul>
          </section>
        )}
      </article>
    </div>
  );
}
