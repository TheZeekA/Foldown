import { describe, expect, it } from "vitest";
import { workspaceNameError } from "./workspaceName";

describe("workspaceNameError", () => {
  it("accepts a single safe folder name", () => {
    expect(workspaceNameError("Project Notes")).toBeNull();
  });

  it("rejects names that cannot be one Windows folder segment", () => {
    for (const value of ["", "   ", ".", "..", "a/b", "a\\b", "a:b", "a*", "trailing.", "trailing "]) {
      expect(workspaceNameError(value), value).not.toBeNull();
    }
  });
});
