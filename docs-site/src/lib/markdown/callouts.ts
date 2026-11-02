import { defineHastPlugin, type HastPluginDefinition } from "satteri";
import { element, text } from "./hast";

const titles = {
  NOTE: "Note",
  TIP: "Tip",
  WARNING: "Warning",
  IMPORTANT: "Important",
} as const;
const marker = /^\[!(NOTE|TIP|WARNING|IMPORTANT)\][ \t]*(?:\r?\n)?/;

/**
 * Turns GitHub alert blockquotes (`> [!NOTE]`) into labelled callouts. The source stays plain
 * Markdown, which GitHub and agents already read; an unknown marker stays a blockquote.
 */
export function callouts(): HastPluginDefinition {
  return defineHastPlugin({
    name: "docs-callouts",
    element: {
      filter: ["blockquote"],
      visit(node, ctx) {
        const paragraph = node.children.find(
          (child) => child.type === "element"
        );
        if (paragraph?.type !== "element" || paragraph.tagName !== "p") return;
        const first = paragraph.children[0];
        const match = first?.type === "text" ? marker.exec(first.value) : null;
        if (first?.type !== "text" || !match?.[1]) return;
        const kind = match[1] as keyof typeof titles;
        const rest = first.value.slice(match[0].length);
        if (rest === "" && paragraph.children.length === 1)
          ctx.removeNode(paragraph);
        else ctx.replaceNode(first, text(rest));
        ctx.insertBefore(
          paragraph,
          element("p", { dataCalloutTitle: "" }, [text(titles[kind])])
        );
        ctx.setProperty(node, "role", "note");
        ctx.setProperty(node, "dataCallout", kind.toLowerCase());
      },
    },
  });
}
