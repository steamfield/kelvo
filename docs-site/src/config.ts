export const SITE = {
  title: "StatGlass",
  description:
    "StatGlass is a free, open-source menu bar system monitor for Apple Silicon Macs. Track CPU, GPU, memory and more, with up to 30 days of local history.",
  /** Landing hero screenshot from brand/build.mjs --og, 2560x1340. */
  ogImage: "/og.webp",
  ogImageAlt:
    "The StatGlass landing page: the headline beside the menu bar popover showing CPU, memory, GPU, power and network.",
  repo: "https://github.com/steamfield/statglass",
  /** Site sections in the header; GitHub and Install sit beside them. */
  nav: [
    { label: "Docs", href: "/docs" },
    { label: "Blog", href: "/blog" },
  ],
} as const;
