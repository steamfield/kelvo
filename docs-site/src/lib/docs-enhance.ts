/**
 * Marks the "On this page" link for the section being read with aria-current="location". Without
 * IntersectionObserver (or JavaScript) the links still work; nothing is marked.
 */
export function spyToc(root: Document): void {
  const links = new Map<string, HTMLAnchorElement>();
  for (const link of root.querySelectorAll<HTMLAnchorElement>(
    "[data-toc-link]"
  )) {
    links.set(decodeURIComponent(link.hash.slice(1)), link);
  }
  const headings = [...links.keys()]
    .map((id) => root.getElementById(id))
    .filter((heading) => heading !== null);
  if (headings.length === 0 || !("IntersectionObserver" in window)) return;

  const visible = new Set<string>();
  const mark = () => {
    const current = headings.find((heading) => visible.has(heading.id))?.id;
    if (current === undefined) return;
    for (const [id, link] of links) {
      if (id === current) link.setAttribute("aria-current", "location");
      else link.removeAttribute("aria-current");
    }
  };
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) visible.add(entry.target.id);
        else visible.delete(entry.target.id);
      }
      mark();
    },
    { rootMargin: "0px 0px -70% 0px" }
  );
  for (const heading of headings) observer.observe(heading);
}
