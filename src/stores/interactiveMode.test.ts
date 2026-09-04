import { beforeEach, describe, expect, it, vi } from "vitest";

const { sendAiMessage, listen } = vi.hoisted(() => ({
  sendAiMessage: vi.fn(),
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("../lib/tauriApi", () => ({
  applyAiProposal: vi.fn(),
  cancelAiRequest: vi.fn(),
  getAiSettings: vi.fn().mockResolvedValue({ provider: "openai", providers: {}, local: {}, cloud: {} }),
  listAiModels: vi.fn(),
  rebuildAiIndex: vi.fn(),
  rejectAiProposal: vi.fn(),
  sendAiMessage,
  setAiSettings: vi.fn(),
}));
vi.mock("../lib/aiProviderConfig", () => ({
  activeProviderConfig: vi.fn().mockReturnValue({ chatModel: null, baseUrl: "", apiKey: null }),
  withActiveProviderConfig: vi.fn(),
}));
vi.mock("./workspace", () => ({
  useWorkspaceStore: { getState: () => ({ refreshTree: vi.fn().mockResolvedValue(undefined) }) },
}));

import { useEditorStore } from "./editor";
import { useInteractiveModeStore } from "./interactiveMode";

describe("interactive mode", () => {
  beforeEach(() => {
    sendAiMessage.mockReset();
    sendAiMessage.mockResolvedValue({ message: "Answer", citations: [], proposals: [], appliedPaths: [] });
    useEditorStore.setState({ openPath: "notes.md", workspaceRoot: "C:\\ws", content: "hello", body: "hello", dirty: false, saveStatus: "idle" });
    useInteractiveModeStore.setState({ messages: [], sending: false, activeRequestId: null, error: null });
  });

  it("does not rewrite a clean open file before reading it for AI chat", async () => {
    const saveNow = vi.fn().mockResolvedValue(undefined);
    useEditorStore.setState({ saveNow });

    await useInteractiveModeStore.getState().send("C:\\ws", "What is this note about?");

    expect(saveNow).not.toHaveBeenCalled();
    expect(sendAiMessage).toHaveBeenCalledOnce();
  });

  it("starts a new chat by clearing the existing conversation state", () => {
    useInteractiveModeStore.setState({
      isOpen: true,
      messages: [{ id: "message-1", role: "user", content: "Old question" }],
      sending: false,
      activeRequestId: "request-1",
      error: "Previous error",
    });

    useInteractiveModeStore.getState().newChat();

    const state = useInteractiveModeStore.getState();
    expect(state.messages).toEqual([]);
    expect(state.sending).toBe(false);
    expect(state.activeRequestId).toBeNull();
    expect(state.error).toBeNull();
    expect(state.isOpen).toBe(true);
  });
});
