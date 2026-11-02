import type { Element, ElementContent, RootContent } from "hast";
import { defineHastPlugin, type HastPluginDefinition } from "satteri";
import { element, hasClass, stringProperty, text } from "./hast";

const copyIcon = element(
  "svg",
  {
    ariaHidden: "true",
    width: "13",
    height: "13",
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: "2",
  },
  [
    element("rect", { x: "8", y: "8", width: "13", height: "13", rx: "2" }),
    element("path", {
      d: "M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3",
    }),
  ]
);

// The markup BaseLayout's [data-copy-command] script enhances: hidden until it runs, result beside the button.
function copyControl(id: string): Element {
  return element("span", { dataCodeCopy: "" }, [
    element(
      "button",
      {
        type: "button",
        hidden: true,
        dataCopyCommand: "",
        ariaLabel: "Copy code",
        ariaControls: [id],
      },
      [copyIcon, element("span", { dataCopyLabel: "" }, [text("Copy")])]
    ),
    element("span", {
      dataCopyResult: "",
      role: "status",
      ariaLive: "polite",
      className: ["sr-only"],
    }),
  ]);
}

/** Frames each highlighted block: a figure with its fence title as caption and a copy button. */
export function codeBlocks(): HastPluginDefinition {
  let count = 0;
  return defineHastPlugin({
    name: "docs-code-blocks",
    element: {
      filter: ["pre"],
      visit(node, ctx) {
        if (!hasClass(node, "astro-code")) return;
        count += 1;
        const id = `code-${count}`;
        const title = stringProperty(node, "data-title");
        const group = stringProperty(node, "data-group");
        const caption = title
          ? [element("figcaption", { dataCodeTitle: "" }, [text(title)])]
          : [];
        ctx.setProperty(node, "id", id);
        ctx.wrapNode(
          node,
          element(
            "figure",
            { dataCodeBlock: "", ...(group && { dataGroup: group }) },
            [...caption, copyControl(id)]
          )
        );
      },
    },
  });
}

function isBlank(node: RootContent | ElementContent | undefined): boolean {
  return node?.type === "text" && node.value.trim() === "";
}

function adjacentElement(
  siblings: readonly (RootContent | ElementContent)[],
  index: number,
  step: 1 | -1
): Element | undefined {
  let at = index + step;
  while (isBlank(siblings[at])) at += step;
  const node = siblings[at];
  return node?.type === "element" ? node : undefined;
}

function groupOf(node: Element | undefined): string | undefined {
  if (node?.tagName !== "figure") return undefined;
  return stringProperty(node, "dataGroup");
}

function runPosition(
  joinsPrevious: boolean,
  joinsNext: boolean
): "first" | "middle" | "last" {
  if (!joinsPrevious) return "first";
  return joinsNext ? "middle" : "last";
}

/**
 * Marks runs of adjacent blocks that share a `group` (a command and its output, before and after)
 * so CSS joins them into one frame. Nothing moves, so without CSS they read as stacked blocks.
 */
export function codeGroups(): HastPluginDefinition {
  return defineHastPlugin({
    name: "docs-code-groups",
    element: {
      filter: ["figure"],
      visit(node, ctx) {
        const group = groupOf(node);
        const index = ctx.indexOf(node);
        if (!group || index === undefined) return;
        const siblings = ctx.parent(node).children;
        const joinsPrevious =
          groupOf(adjacentElement(siblings, index, -1)) === group;
        const joinsNext =
          groupOf(adjacentElement(siblings, index, 1)) === group;
        if (!joinsPrevious && !joinsNext) return;
        ctx.setProperty(
          node,
          "dataGroupPosition",
          runPosition(joinsPrevious, joinsNext)
        );
      },
    },
  });
}
