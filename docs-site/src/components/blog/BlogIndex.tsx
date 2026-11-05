import type { BlogEntry } from "@/lib/blog";
import { blogPath } from "@/lib/blog";

export function BlogIndex({ posts }: { posts: readonly BlogEntry[] }) {
  return (
    <section className="mx-auto max-w-[44rem] pt-8 sm:pt-12">
      <header className="mb-10 space-y-4">
        <h1 className="text-3xl font-[590] tracking-tight sm:text-4xl">Blog</h1>
        <p className="text-base leading-relaxed text-pretty text-secondary-foreground">
          Mac system monitoring guides, with the app here to try as you read.
        </p>
      </header>
      <ul className="divide-y divide-border border-y border-border">
        {posts.map((post) => (
          <li key={post.id} className="space-y-3 py-6">
            <h2 className="text-xl font-[590] tracking-tight">
              <a href={blogPath(post.id)} className="hover:text-link">
                {post.data.title}
              </a>
            </h2>
            <p className="text-base leading-relaxed text-pretty text-secondary-foreground">
              {post.data.description}
            </p>
            <time
              className="text-xs text-muted-foreground"
              dateTime={post.data.published.toISOString()}
            >
              {post.data.published.toLocaleDateString("en-US", {
                timeZone: "UTC",
                year: "numeric",
                month: "long",
                day: "numeric",
              })}
            </time>
          </li>
        ))}
      </ul>
    </section>
  );
}
