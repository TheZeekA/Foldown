import type { SelectionAiAction } from "../../lib/types";

export const SELECTION_AI_ACTIONS: { value: SelectionAiAction; label: string }[] = [
  { value: "explain", label: "Explain" },
  { value: "summarize", label: "Summarize" },
  { value: "rewrite", label: "Rewrite" },
  { value: "clarify", label: "Improve clarity" },
  { value: "checklist", label: "Convert to checklist" },
  { value: "action-items", label: "Extract action items" },
  { value: "translate", label: "Translate" },
];

const INSTRUCTIONS: Record<SelectionAiAction, string> = {
  explain: "Explain the selected passage clearly.",
  summarize: "Summarize the selected passage while preserving its important meaning.",
  rewrite: "Rewrite the selected passage while preserving its meaning and Markdown structure.",
  clarify: "Improve the clarity and readability of the selected passage without changing its meaning.",
  checklist: "Convert the selected passage into a concise Markdown checklist.",
  "action-items": "Extract concrete action items from the selected passage as a Markdown checklist.",
  translate: "Translate the selected passage. Preserve Markdown structure and infer the target language from the user's request if one is provided.",
};

export function buildSelectionPrompt(action: SelectionAiAction, selectedText: string): string {
  const clean = selectedText.trim();
  if (!clean) throw new Error("Select some text before using an AI action");
  return `${INSTRUCTIONS[action]}\n\nSelected Markdown:\n---\n${clean}\n---`;
}

