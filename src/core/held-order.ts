/** Keep held identities in place, drop departed rows and append newcomers. */
export function holdOrder<T>(
  sorted: readonly T[],
  held: readonly string[],
  getId: (row: T) => string
): T[] {
  const byId = new Map(sorted.map((row) => [getId(row), row]));
  const placed = new Set(held);
  return [
    ...held.flatMap((id) => {
      const row = byId.get(id);
      return row === undefined ? [] : [row];
    }),
    ...sorted.filter((row) => !placed.has(getId(row))),
  ];
}
