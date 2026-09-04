import { describe, expect, it } from "vitest";
import { formatUpdateCheckError, formatUpdateDetails } from "./updater";

describe("formatUpdateDetails", () => {
  it("describes the available version and release notes", () => {
    expect(formatUpdateDetails({ version: "1.2.1", body: "Bug fixes", date: undefined }))
      .toBe("Version 1.2.1 is available.\n\nBug fixes");
  });

  it("uses a useful fallback when release notes are absent", () => {
    expect(formatUpdateDetails({ version: "1.2.1", body: undefined, date: undefined }))
      .toBe("Version 1.2.1 is available.");
  });

  it("does not create a user-facing message for a background launch check", () => {
    expect(formatUpdateCheckError(new Error("network unavailable"), false)).toBeNull();
  });

  it("keeps a useful error for an explicit About check", () => {
    expect(formatUpdateCheckError(new Error("network unavailable"), true))
      .toBe("Could not check for updates: Error: network unavailable");
  });
});
