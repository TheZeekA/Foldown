import { describe, expect, it } from "vitest";
import { candidateLocalEndpoints, findProbe, formatProbeOption } from "./localServerDiscovery";

describe("candidateLocalEndpoints", () => {
  it("returns a fixed list of common local server addresses", () => {
    const endpoints = candidateLocalEndpoints();
    expect(endpoints).toContain("http://127.0.0.1:9932/v1");
    expect(endpoints).toContain("http://127.0.0.1:9933/v1");
    expect(endpoints.every((url) => url.startsWith("http://127.0.0.1:"))).toBe(true);
  });
});

describe("formatProbeOption", () => {
  it("shows the address and first model", () => {
    expect(formatProbeOption({ baseUrl: "http://127.0.0.1:9932/v1", models: ["nomic-embed-text-v1.5"] })).toBe(
      "127.0.0.1:9932 — nomic-embed-text-v1.5",
    );
  });

  it("falls back to just the address when no models were reported", () => {
    expect(formatProbeOption({ baseUrl: "http://127.0.0.1:9932/v1", models: [] })).toBe("127.0.0.1:9932");
  });

  it("truncates a very long model identifier, such as a full file path", () => {
    const longPath = "C:\\llama\\models\\some-very-long-nomic-embed-text-v1.5-quantized-filename.gguf";
    const label = formatProbeOption({ baseUrl: "http://127.0.0.1:9932/v1", models: [longPath] });
    expect(label.length).toBeLessThan(longPath.length);
    expect(label).toContain("…");
  });
});

describe("findProbe", () => {
  it("finds a probe result by its exact base URL", () => {
    const probes = [{ baseUrl: "http://127.0.0.1:9931/v1", models: ["a"] }, { baseUrl: "http://127.0.0.1:9932/v1", models: ["b"] }];
    expect(findProbe(probes, "http://127.0.0.1:9932/v1")?.models).toEqual(["b"]);
  });

  it("returns undefined when no probe matched that URL", () => {
    expect(findProbe([{ baseUrl: "http://127.0.0.1:9931/v1", models: ["a"] }], "http://127.0.0.1:9932/v1")).toBeUndefined();
  });
});
