export const SITE = {
  title: "StatGlass",
  description:
    "A menu bar system monitor for Apple Silicon Macs that keeps history.",
  /** Landing hero screenshot from brand/build.mjs --og, 2560x1340. */
  ogImage: "/og.webp",
  ogImageAlt:
    "The StatGlass landing page: the headline beside the menu bar popover showing CPU, memory, GPU, power and network.",
  repo: "https://github.com/steamfield/statglass",
  /** Site sections in the header; GitHub and Install sit beside them. */
  nav: [{ label: "Docs", href: "/docs" }],
} as const;
