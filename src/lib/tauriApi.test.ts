import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { sendAiMessage } from "./tauriApi";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("sendAiMessage", () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it("sends the active document path for complete edit context", async () => {
    vi.mocked(invoke).mockResolvedValue({ message: "", citations: [], proposals: [], appliedPaths: [] });

    await sendAiMessage("C:\\Notes", "request-1", [{ role: "user", content: "Remove section 4" }], "C:\\Notes\\Code Review.md");

    expect(invoke).toHaveBeenCalledWith("send_ai_message", {
      workspaceRoot: "C:\\Notes",
      requestId: "request-1",
      messages: [{ role: "user", content: "Remove section 4" }],
      activePath: "C:\\Notes\\Code Review.md",
    });
  });
});
