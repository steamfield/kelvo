import type { DocLink } from "@/lib/docs";
import styles from "./DocCard.module.css";

/** A link card with a tinted corner. `accent` picks a module accent, 1 to 6. */
export function DocCard({
  link,
  accent = 1,
}: {
  link: DocLink;
  accent?: number;
}) {
  return (
    <a
      href={link.href}
      data-accent={accent}
      className={`${styles.card} block rounded-card border border-border p-5 no-underline`}
    >
      <span className="block text-sm font-[590] tracking-tight text-foreground">
        {link.label}
      </span>
      <span className="mt-1 block text-[13px] leading-relaxed text-muted-foreground">
        {link.description}
      </span>
    </a>
  );
}

export function DocCardGrid({ links }: { links: readonly DocLink[] }) {
  if (links.length === 0) return null;
  return (
    <ul className="grid gap-4 sm:grid-cols-2">
      {links.map((link, index) => (
        <li key={link.id}>
          <DocCard link={link} accent={(index % 6) + 1} />
        </li>
      ))}
    </ul>
  );
}
