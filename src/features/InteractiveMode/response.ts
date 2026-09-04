export function splitAssistantResponse(text: string): { message: string; hasActionBlock: boolean } {
  const canonical = /```foldown-actions\b/i.exec(text);
  const generic = /```json\b/i.exec(text);
  const genericPayload = generic ? text.slice(generic.index + generic[0].length) : "";
  const genericIsActions = /"actions?"\s*:|"type"\s*:\s*"(?:create|replace|delete)"/i.test(genericPayload);
  const match = canonical ?? (genericIsActions ? generic : null);
  if (!match) return { message: text.trim(), hasActionBlock: false };
  return { message: text.slice(0, match.index).trim(), hasActionBlock: true };
}

export function endpointHost(baseUrl: string): string {
  try { return new URL(baseUrl).host || "Invalid endpoint"; }
  catch { return "Invalid endpoint"; }
}
