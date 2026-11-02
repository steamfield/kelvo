import GithubSlugger from "github-slugger";
import { defineHastPlugin, type HastPluginDefinition } from "satteri";
import { element, stringProperty } from "./hast";

/**
 * Gives section headings their ids and a hover link. Astro's own id pass runs after user plugins
 * and keeps an existing id, so the ids, the links and the table of contents agree.
 */
export function headingAnchors(): HastPluginDefinition {
  const slugger = new GithubSlugger();
  return defineHastPlugin({
    name: "docs-heading-anchors",
    element: {
      filter: ["h2", "h3"],
      visit(node, ctx) {
        const id =
          stringProperty(node, "id") ?? slugger.slug(ctx.textContent(node));
        ctx.setProperty(node, "id", id);
        ctx.appendChild(
          node,
          element("a", {
            href: `#${id}`,
            dataHeadingAnchor: "",
            ariaLabel: `Link to section: ${ctx.textContent(node)}`,
          })
        );
      },
    },
  });
}
