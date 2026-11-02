import type { DocNavGroup } from "../../lib/docs";

function DocsNav({
  nav,
  current,
  idPrefix,
}: {
  nav: DocNavGroup[];
  current: string;
  idPrefix: string;
}) {
  return (
    <nav aria-label="Documentation">
      {nav.map((group, index) => {
        const headingId = `${idPrefix}-${group.title.toLowerCase().replaceAll(" ", "-")}`;
        return (
          <div
            key={group.title}
            className={
              index > 0 ? "mt-4 border-t border-border/70 pt-4" : undefined
            }
          >
            <p
              id={headingId}
              className="mb-2 text-[11px] text-secondary-foreground"
            >
              {group.title}
            </p>
            <ul aria-labelledby={headingId} className="space-y-px">
              {group.items.map((item) => {
                const active = item.id === current;
                return (
                  <li key={item.id}>
                    <a
                      href={item.href}
                      aria-current={active ? "page" : undefined}
                      className={`block rounded-control px-3 py-1.5 text-sm ${active ? "bg-secondary font-[510] text-foreground" : "text-muted-foreground hover:text-foreground"}`}
                    >
                      {item.label}
                    </a>
                  </li>
                );
              })}
            </ul>
          </div>
        );
      })}
    </nav>
  );
}

/**
 * The docs navigation: a disclosure on small screens, so it opens without JavaScript, and a
 * sticky column on wide ones. Two copies because a closed `<details>` keeps its links out of the
 * tab order even when CSS shows them; only one is ever displayed.
 */
export function DocsSidebar({
  nav,
  current,
}: {
  nav: DocNavGroup[];
  current: string;
}) {
  return (
    <div>
      <details data-docs-menu className="lg:hidden">
        <summary className="flex cursor-pointer items-center rounded-control border border-border bg-card px-3 py-2 text-sm font-[510] text-foreground">
          Docs menu
        </summary>
        <div className="mt-3">
          <DocsNav nav={nav} current={current} idPrefix="docs-menu" />
        </div>
      </details>
      <div
        data-docs-sidebar
        className="sticky top-24 hidden max-h-[calc(100vh-7rem)] overflow-y-auto pb-8 lg:block"
      >
        <DocsNav nav={nav} current={current} idPrefix="docs-nav" />
      </div>
    </div>
  );
}
