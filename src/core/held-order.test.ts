import { holdOrder } from "./held-order";

describe("holdOrder", () => {
  const a = { id: "a", value: 1 };
  const b = { id: "b", value: 2 };
  const c = { id: "c", value: 3 };
  const id = (row: typeof a) => row.id;

  it("keeps held order, drops gone rows and appends new rows in sorted order", () => {
    expect(holdOrder([c, a], ["a", "b"], id)).toEqual([a, c]);
    expect(holdOrder([c, b, a], ["b", "gone"], id)).toEqual([b, c, a]);
  });

  it("uses the latest row values without mutating either input", () => {
    const updated = { ...a, value: 9 };
    const sorted = Object.freeze([b, updated]);
    const held = Object.freeze(["a", "b"]);
    expect(holdOrder(sorted, held, id)).toEqual([updated, b]);
    expect(sorted).toEqual([b, updated]);
    expect(held).toEqual(["a", "b"]);
  });

  it("keeps sorted order without a hold and handles empty or departed rows", () => {
    expect(holdOrder([b, a], [], id)).toEqual([b, a]);
    expect(holdOrder([], ["a"], id)).toEqual([]);
    expect(holdOrder([a], ["gone"], id)).toEqual([a]);
  });
});
