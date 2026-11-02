/**
 * A copy control for the element named by `id`. It renders hidden and
 * `enhanceCopyButtons` (src/lib/copy-buttons.ts) reveals it once scripts run,
 * so pages without JavaScript never show a button that does nothing.
 */
export function CopyButton({
  id,
  label = "Copy",
}: {
  id: string;
  label?: string;
}) {
  return (
    <span className="inline-flex shrink-0 items-center">
      <button
        hidden
        type="button"
        data-copy-command
        aria-label={label}
        aria-controls={id}
        className="flex shrink-0 items-center gap-1.5 rounded-control border border-border bg-card px-2.5 py-1.5 text-xs font-[510] whitespace-nowrap text-foreground hover:bg-secondary transition-transform duration-(--motion-fast) ease-(--ease-out) active:scale-95"
      >
        <svg
          aria-hidden="true"
          width="13"
          height="13"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <rect x="8" y="8" width="13" height="13" rx="2" />
          <path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3" />
        </svg>
        <span data-copy-label>{label}</span>
      </button>
      <span
        data-copy-result
        role="status"
        aria-live="polite"
        className="sr-only"
      />
    </span>
  );
}
