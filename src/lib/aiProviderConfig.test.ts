import { describe, expect, it } from "vitest";
import { activeProviderConfig, PROVIDER_LABELS, withActiveProviderConfig } from "./aiProviderConfig";
import type { AiSettings } from "./types";

function baseSettings(): AiSettings {
  return {
    provider: "openai",
    local: { baseUrl: "http://localhost:11434/v1", chatModel: "qwen3:8b", apiKey: null },
    openai: { baseUrl: "https://api.openai.com/v1", chatModel: "gpt-4.1", apiKey: "sk-test" },
    anthropic: { baseUrl: "https://api.anthropic.com/v1", chatModel: "", apiKey: null },
    gemini: { baseUrl: "https://generativelanguage.googleapis.com/v1beta", chatModel: "", apiKey: null },
    embeddingModel: null,
    embeddingBaseUrl: null,
    embeddingDocumentPrefix: "search_document: ",
    embeddingQueryPrefix: "search_query: ",
    retrievalCandidateCount: 20,
    retrievalFinalCount: 8,
    retrievalMaxChars: 12000,
    rerankerEnabled: false,
    rerankerBaseUrl: null,
    rerankerModel: null,
  };
}

describe("activeProviderConfig", () => {
  it("returns the block matching the current provider selection", () => {
    expect(activeProviderConfig(baseSettings())).toEqual({ baseUrl: "https://api.openai.com/v1", chatModel: "gpt-4.1", apiKey: "sk-test" });
  });

  it("follows a switched provider selection", () => {
    const settings = { ...baseSettings(), provider: "gemini" as const };
    expect(activeProviderConfig(settings).baseUrl).toBe("https://generativelanguage.googleapis.com/v1beta");
  });
});

describe("withActiveProviderConfig", () => {
  it("replaces only the active provider's block, leaving the other three untouched", () => {
    const settings = baseSettings();
    const updated = withActiveProviderConfig(settings, { ...settings.openai, chatModel: "gpt-4.1-mini" });
    expect(updated.openai.chatModel).toBe("gpt-4.1-mini");
    expect(updated.local).toEqual(settings.local);
    expect(updated.anthropic).toEqual(settings.anthropic);
    expect(updated.gemini).toEqual(settings.gemini);
  });

  it("targets whichever provider is currently active, not always openai", () => {
    const settings = { ...baseSettings(), provider: "local" as const };
    const updated = withActiveProviderConfig(settings, { ...settings.local, chatModel: "llama3.2:latest" });
    expect(updated.local.chatModel).toBe("llama3.2:latest");
    expect(updated.openai).toEqual(settings.openai);
  });
});

describe("PROVIDER_LABELS", () => {
  it("has a human-readable label for every provider", () => {
    expect(PROVIDER_LABELS.local).toBe("Local Server");
    expect(PROVIDER_LABELS.openai).toBe("ChatGPT");
    expect(PROVIDER_LABELS.anthropic).toBe("Claude");
    expect(PROVIDER_LABELS.gemini).toBe("Gemini");
  });
});
