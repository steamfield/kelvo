import type { Element, ElementContent, Properties } from "hast";

export function element(
  tagName: string,
  properties: Properties,
  children: ElementContent[] = []
): Element {
  return { type: "element", tagName, properties, children };
}

export function text(value: string): ElementContent {
  return { type: "text", value };
}

export function stringProperty(
  node: Element,
  name: string
): string | undefined {
  const value = node.properties[name];
  return typeof value === "string" && value !== "" ? value : undefined;
}

export function hasClass(node: Element, name: string): boolean {
  const value = node.properties.class ?? node.properties.className;
  const classes = Array.isArray(value)
    ? value.map(String)
    : String(value ?? "").split(/\s+/);
  return classes.includes(name);
}
