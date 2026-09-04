import { describe, expect, it } from "vitest";
import { ABOUT_DEVELOPER, ABOUT_EMAIL, formatVersion } from "./aboutMetadata";

describe("about metadata", () => {
  it("exports the approved developer and support email", () => {
    expect(ABOUT_DEVELOPER).toBe("Zeeka Limited");
    expect(ABOUT_EMAIL).toBe("support@zeeka.nz");
  });

  it("formats a running version", () => {
    expect(formatVersion("0.1.0")).toBe("Version 0.1.0");
  });

  it("trims a running version", () => {
    expect(formatVersion(" 0.1.0 ")).toBe("Version 0.1.0");
  });

  it("reports an unavailable version for empty input", () => {
    expect(formatVersion("  ")).toBe("Version unavailable");
  });
});
