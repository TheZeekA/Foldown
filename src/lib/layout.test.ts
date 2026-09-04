import { describe, expect, it } from "vitest";
import { clampRange } from "./layout";

describe("layout helpers", () => {
  it("keeps a dragged panel size inside its allowed range", () => {
    expect(clampRange(150, 200, 420)).toBe(200);
    expect(clampRange(320, 200, 420)).toBe(320);
    expect(clampRange(500, 200, 420)).toBe(420);
  });
});
