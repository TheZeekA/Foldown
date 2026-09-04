import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import type { AiActionProposal, AiChatMessage, AiContextChunk, AiProvider, AiSettings } from "../lib/types";
import { applyAiProposal, cancelAiRequest, getAiSettings, listAiModels, rebuildAiIndex, rejectAiProposal, sendAiMessage, setAiSettings } from "../lib/tauriApi";
import { activeProviderConfig, withActiveProviderConfig } from "../lib/aiProviderConfig";
import { useWorkspaceStore } from "./workspace";
import { useEditorStore } from "./editor";

export interface DisplayMessage extends AiChatMessage {
  id: string;
  citations?: AiContextChunk[];
  proposals?: AiActionProposal[];
}

interface InteractiveModeState {
  isOpen: boolean;
  settings: AiSettings | null;
  models: string[];
  messages: DisplayMessage[];
  sending: boolean;
  indexing: boolean;
  indexStatus: "idle" | "indexing" | "ready" | "error";
  activeRequestId: string | null;
  error: string | null;
  open: () => Promise<void>;
  close: () => void;
  newChat: () => void;
  saveSettings: (settings: AiSettings) => Promise<void>;
  fetchModels: (provider: AiProvider, baseUrl: string, apiKey: string | null) => Promise<string[]>;
  switchModel: (model: string) => Promise<void>;
  send: (workspaceRoot: string, content: string) => Promise<void>;
  cancel: () => Promise<void>;
  rebuildIndex: (workspaceRoot: string) => Promise<void>;
  setIndexStatus: (status: "indexing" | "ready" | "error", detail: string | null) => void;
  approve: (proposalId: string) => Promise<string>;
  reject: (proposalId: string) => Promise<void>;
  resetForWorkspace: () => void;
}

const id = () => globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;

export const useInteractiveModeStore = create<InteractiveModeState>((set, get) => ({
  isOpen: false, settings: null, models: [], messages: [], sending: false, indexing: false,
  indexStatus: "idle",
  activeRequestId: null, error: null,
  open: async () => {
    set({ isOpen: true, error: null });
    if (!get().settings) {
      try {
        const settings = await getAiSettings();
        set({ settings });
        const active = activeProviderConfig(settings);
        if (active.chatModel) void get().fetchModels(settings.provider, active.baseUrl, active.apiKey);
      }
      catch (error) { set({ error: String(error) }); }
    }
  },
  close: () => set({ isOpen: false }),
  newChat: () => set({ messages: [], sending: false, activeRequestId: null, error: null }),
  saveSettings: async (settings) => {
    await setAiSettings(settings);
    set({ settings, error: null });
  },
  fetchModels: async (provider, baseUrl, apiKey) => {
    const models = await listAiModels(provider, baseUrl, apiKey);
    set({ models, error: null });
    return models;
  },
  switchModel: async (model) => {
    const settings = get().settings;
    if (!settings || !model) return;
    const next = withActiveProviderConfig(settings, { ...activeProviderConfig(settings), chatModel: model });
    await setAiSettings(next);
    set({ settings: next });
  },
  send: async (workspaceRoot, content) => {
    const clean = content.trim();
    if (!clean || get().sending) return;
    const user: DisplayMessage = { id: id(), role: "user", content: clean };
    const requestId = id();
    const assistantId = id();
    const messages = [...get().messages, user];
    set({ messages: [...messages, { id: assistantId, role: "assistant", content: "" }], sending: true, activeRequestId: requestId, error: null });
    const unlisten = await listen<{ requestId: string; delta: string }>("ai-chat-delta", (event) => {
      if (event.payload.requestId !== requestId) return;
      set((state) => ({ messages: state.messages.map((message) => message.id === assistantId ? { ...message, content: message.content + event.payload.delta } : message) }));
    });
    try {
      const editor = useEditorStore.getState();
      if (editor.openPath && editor.dirty) await editor.saveNow();
      const history = messages.map(({ role, content }) => ({ role, content }));
      const result = await sendAiMessage(workspaceRoot, requestId, history, editor.openPath);
      if (result.appliedPaths.length) {
        await useWorkspaceStore.getState().refreshTree();
        if (useEditorStore.getState().openPath && result.appliedPaths.includes(useEditorStore.getState().openPath!)) {
          await useEditorStore.getState().reloadFromDisk();
        }
      }
      set((state) => ({ messages: state.messages.map((message) => message.id === assistantId ? { ...message, content: result.message, citations: result.citations, proposals: result.proposals } : message), sending: false, activeRequestId: null }));
    } catch (error) {
      set((state) => ({ messages: state.messages.filter((message) => message.id !== assistantId || !!message.content), sending: false, activeRequestId: null, error: String(error) }));
      // The failed request may still have written files to disk before erroring
      // (e.g. a partially-applied "actions" response) — refresh the tree so it
      // never silently goes stale. Don't force-reload the open file's content
      // here: that would risk discarding edits the user typed after the
      // pre-send save, so leave that to the normal file-watcher/"external
      // change" flow, which only ever prompts rather than overwriting.
      await useWorkspaceStore.getState().refreshTree();
    }
    finally { unlisten(); }
  },
  cancel: async () => {
    const requestId = get().activeRequestId;
    if (requestId) await cancelAiRequest(requestId);
  },
  rebuildIndex: async (workspaceRoot) => {
    set({ indexing: true, error: null });
    try { await rebuildAiIndex(workspaceRoot); set({ indexing: false }); }
    catch (error) { set({ indexing: false, error: String(error) }); }
  },
  setIndexStatus: (status, detail) => set({ indexStatus: status, error: status === "error" ? detail : null }),
  approve: async (proposalId) => applyAiProposal(proposalId),
  reject: async (proposalId) => { await rejectAiProposal(proposalId); },
  resetForWorkspace: () => set({ messages: [], sending: false, activeRequestId: null, error: null, indexStatus: "idle" }),
}));
