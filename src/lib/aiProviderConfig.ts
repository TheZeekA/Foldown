import type { AiProvider, AiSettings, ProviderConfig } from "./types";

export const PROVIDER_LABELS: Record<AiProvider, string> = {
  local: "Local Server",
  openai: "ChatGPT",
  anthropic: "Claude",
  gemini: "Gemini",
};

/** The `ProviderConfig` block matching `settings.provider` — the four block
 * field names (`local`/`openai`/`anthropic`/`gemini`) are exactly the four
 * `AiProvider` string values, so this is a direct index rather than a
 * switch. */
export function activeProviderConfig(settings: AiSettings): ProviderConfig {
  return settings[settings.provider];
}

/** Returns a copy of `settings` with only the currently-active provider's
 * block replaced — the other three blocks (and everything else typed into
 * them) are untouched. */
export function withActiveProviderConfig(settings: AiSettings, config: ProviderConfig): AiSettings {
  return { ...settings, [settings.provider]: config };
}
