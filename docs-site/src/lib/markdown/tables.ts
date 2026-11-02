import { defineHastPlugin, type HastPluginDefinition } from "satteri";
import { element } from "./hast";

/**
 * Wraps tables in a focusable scroll box, so a wide table scrolls by itself on a phone, and marks
 * header cells as column headers.
 */
export function tables(): HastPluginDefinition {
  return defineHastPlugin({
    name: "docs-tables",
    element: [
      {
        filter: ["table"],
        visit(node, ctx) {
          ctx.wrapNode(
            node,
            element("div", { dataTableScroll: "", tabIndex: 0 })
          );
        },
      },
      {
        filter: ["th"],
        visit(node, ctx) {
          ctx.setProperty(node, "scope", "col");
        },
      },
    ],
  });
}
