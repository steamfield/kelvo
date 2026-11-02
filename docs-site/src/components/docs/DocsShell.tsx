import type { ReactNode } from "react";
import type { DocLink, DocNavGroup } from "../../lib/docs";
import { DocCardGrid } from "./DocCard";
import { DocsPageActions } from "./DocsPageActions";
import { DocsPager } from "./DocsPager";
import prose from "./DocsProse.module.css";
import { DocsSidebar } from "./DocsSidebar";
import { type DocHeading, TableOfContents } from "./TableOfContents";

export type DocsPageView = {
  id: string;
  title: string;
  description: string;
  group?: string | undefined;
  markdown: string;
  markdownHref: string;
  markdownUrl: string;
};

/**
 * The docs page frame: navigation, the article with its actions and
 * previous/next links, and "On this page". `children` is the rendered Markdown.
 */
export function DocsShell({
  nav,
  page,
  headings,
  featured = [],
  previous,
  next,
  children,
}: {
  nav: DocNavGroup[];
  page: DocsPageView;
  headings: readonly DocHeading[];
  featured?: readonly DocLink[];
  previous?: DocLink | undefined;
  next?: DocLink | undefined;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-8 lg:grid-cols-[11rem_minmax(0,1fr)] lg:gap-12 xl:grid-cols-[11rem_minmax(0,1fr)_12rem]">
      <DocsSidebar nav={nav} current={page.id} />
      <article className="min-w-0 max-w-[50rem]">
        <header className="space-y-3 border-b border-border pb-6">
          <nav
            aria-label="Breadcrumb"
            className="text-xs text-muted-foreground"
          >
            <ol className="flex flex-wrap items-center gap-1.5">
              <li>
                <a
                  href="/docs"
                  className="text-muted-foreground hover:text-link"
                >
                  Docs
                </a>
              </li>
              {page.group && (
                <li className="before:mr-1.5 before:content-['/']">
                  {page.group}
                </li>
              )}
            </ol>
          </nav>
          <h1 className="text-3xl font-[590] tracking-tight text-foreground sm:text-4xl">
            {page.title}
          </h1>
          <p className="text-base leading-7 text-muted-foreground sm:text-lg">
            {page.description}
          </p>
          <DocsPageActions
            markdown={page.markdown}
            markdownHref={page.markdownHref}
            markdownUrl={page.markdownUrl}
          />
        </header>
        <div className={`${prose.prose} mt-8`}>{children}</div>
        {featured.length > 0 && (
          <section aria-labelledby="docs-start-here" className="mt-12">
            <h2
              id="docs-start-here"
              className="mb-4 text-xl font-[590] tracking-tight text-foreground"
            >
              Start here
            </h2>
            <DocCardGrid links={featured} />
          </section>
        )}
        <DocsPager previous={previous} next={next} />
      </article>
      <div className="hidden xl:block">
        <TableOfContents headings={headings} />
      </div>
    </div>
  );
}
