import { describe, expect, it } from "vitest";
import { buildHistoryDiff } from "./historyDiff";

describe("buildHistoryDiff", () => {
  it("marks unchanged, removed, and added lines", () => {
    expect(buildHistoryDiff("one\nnew", "one\nold")).toEqual([
      { kind: "same", text: "one" },
      { kind: "removed", text: "old" },
      { kind: "added", text: "new" },
    ]);
  });
});

