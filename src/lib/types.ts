export type TreeNode =
  | { type: "file"; name: string; path: string }
  | { type: "folder"; name: string; path: string; children: TreeNode[] };

export interface SearchResult {
  path: string;
  name: string;
  snippet: string;
}

export type ThemeMode = "system" | "light" | "dark";

export interface EditorFont {
  family: string;
  size: number;
}

export interface RecentWorkspace {
  path: string;
  name: string;
  lastOpened: number;
  available: boolean;
}

export type AiProvider = "local" | "openai" | "anthropic" | "gemini";

export interface ProviderConfig {
  baseUrl: string;
  chatModel: string;
  apiKey: string | null; // merged in from Windows Credential Manager on read
}

export interface AiSettings {
  provider: AiProvider;
  local: ProviderConfig;
  openai: ProviderConfig;
  anthropic: ProviderConfig;
  gemini: ProviderConfig;

  // unchanged RAG fields — always local, regardless of chat provider
  embeddingModel: string | null;
  embeddingBaseUrl: string | null;
  embeddingDocumentPrefix: string;
  embeddingQueryPrefix: string;
  retrievalCandidateCount: number;
  retrievalFinalCount: number;
  retrievalMaxChars: number;
  rerankerEnabled: boolean;
  rerankerBaseUrl: string | null;
  rerankerModel: string | null;
}

export interface AiServerProbe {
  baseUrl: string;
  models: string[];
}

export interface AiChatMessage {
  role: "user" | "assistant";
  content: string;
}

export interface AiContextChunk {
  path: string;
  heading: string;
  text: string;
  score: number;
  ordinal: number;
}

export interface AiActionProposal {
  id: string;
  actionType: "create" | "replace" | "delete";
  path: string;
  oldContent: string | null;
  newContent: string | null;
}

export interface AiChatResult {
  message: string;
  citations: AiContextChunk[];
  proposals: AiActionProposal[];
  appliedPaths: string[];
}

export type SelectionAiAction = "explain" | "summarize" | "rewrite" | "clarify" | "checklist" | "action-items" | "translate";

export interface SelectionAiResult {
  text: string;
  citations: AiContextChunk[];
}

export interface BulkConvertResult {
  source_path: string;
  dest_path: string | null;
  error: string | null;
}
