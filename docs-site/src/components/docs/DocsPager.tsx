import type { DocLink } from "../../lib/docs";

function PagerLink({
  link,
  direction,
}: {
  link: DocLink;
  direction: "Previous" | "Next";
}) {
  return (
    <a
      href={link.href}
      className={`block rounded-card border border-border bg-card p-4 no-underline hover:border-link ${direction === "Next" ? "text-right sm:col-start-2" : ""}`}
    >
      <span className="block text-xs text-muted-foreground">{direction}</span>
      <span className="mt-1 block text-sm font-[510] text-foreground">
        {link.label}
      </span>
    </a>
  );
}

export function DocsPager({
  previous,
  next,
}: {
  previous?: DocLink | undefined;
  next?: DocLink | undefined;
}) {
  if (!previous && !next) return null;
  return (
    <nav
      aria-label="Previous and next pages"
      className="mt-14 grid gap-3 sm:grid-cols-2"
    >
      {previous && <PagerLink link={previous} direction="Previous" />}
      {next && <PagerLink link={next} direction="Next" />}
    </nav>
  );
}
