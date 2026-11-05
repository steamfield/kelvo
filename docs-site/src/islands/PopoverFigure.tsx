import { MenuBarItem } from "@/islands/MenuBarItem";
import { PopoverDemo } from "@/islands/PopoverDemo";

export type PopoverFigureProps = { caption: string };

export function PopoverFigure({ caption }: PopoverFigureProps) {
  return (
    <figure
      data-demo=""
      data-nosnippet=""
      className="mx-auto my-8 w-full max-w-[360px] text-base leading-normal font-[510]"
    >
      <div className="flex flex-col gap-1.5">
        <MenuBarItem />
        <PopoverDemo />
      </div>
      <figcaption className="mt-3 text-xs text-muted-foreground">
        {caption}
      </figcaption>
    </figure>
  );
}
