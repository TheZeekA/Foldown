import type { AiServerProbe } from "../../lib/types";

/** Ports commonly used by local OpenAI-compatible model servers (llama.cpp,
 * Ollama, LM Studio, and this app's own suggested embedding/reranker ports)
 * — scanned on localhost so a user doesn't have to know or type the address
 * of a server they already have running. */
const COMMON_LOCAL_PORTS = [8080, 9932, 9933, 11434, 1234, 8000, 5000];

export function candidateLocalEndpoints(): string[] {
  return COMMON_LOCAL_PORTS.map((port) => `http://127.0.0.1:${port}/v1`);
}

/** A short human-readable label for one discovered server, e.g.
 * "127.0.0.1:9932 — nomic-embed-text-v1.5" (falls back to just the address
 * if it somehow reported zero models, and truncates a very long model
 * identifier — llama.cpp often reports the full file path as the model id). */
export function formatProbeOption(probe: AiServerProbe): string {
  const address = probe.baseUrl.replace(/^https?:\/\//, "").replace(/\/v1\/?$/, "");
  const model = probe.models[0];
  if (!model) return address;
  const label = model.length > 60 ? `…${model.slice(-57)}` : model;
  return `${address} — ${label}`;
}

export function findProbe(probes: AiServerProbe[], baseUrl: string): AiServerProbe | undefined {
  return probes.find((probe) => probe.baseUrl === baseUrl);
}
