import { DashboardDemo } from "@/islands/DashboardDemo";
import { ModuleLiveCard, type ModuleKey } from "@/islands/ModuleLiveCard";
import type { DemoParams } from "@/lib/demo";

export type ArticleDemoProps = {
  route: string;
  params?: DemoParams;
  title: string;
  caption: string;
  fallback?: ModuleKey;
};

export function ArticleDemo({
  route,
  params,
  title,
  caption,
  fallback,
}: ArticleDemoProps) {
  return (
    <figure
      data-nosnippet=""
      data-demo=""
      // 100cqw is BlogArticle's full-width wrapper: the page column without the scrollbar.
      className="relative left-1/2 my-8 w-[100cqw] max-w-[1136px] -translate-x-1/2 text-base leading-normal"
    >
      {/* Match DashboardDemo's DASHBOARD_MIN_WIDTH + 64 fit query, including before hydration. */}
      {fallback && (
        <div className="mb-4 min-[1088px]:hidden">
          <ModuleLiveCard module={fallback} />
        </div>
      )}
      <DashboardDemo route={route} params={params ?? {}} title={title} />
      <figcaption className="mt-3 text-xs text-muted-foreground">
        {caption}
      </figcaption>
    </figure>
  );
}
