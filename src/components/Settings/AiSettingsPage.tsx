import { useEffect, useState } from "react";
import { getAiSettings, listAiModels, probeAiEndpoints } from "../../lib/tauriApi";
import type { AiProvider, AiServerProbe, AiSettings, ProviderConfig } from "../../lib/types";
import { activeProviderConfig, PROVIDER_LABELS, withActiveProviderConfig } from "../../lib/aiProviderConfig";
import { useInteractiveModeStore } from "../../stores/interactiveMode";
import { candidateLocalEndpoints, findProbe, formatProbeOption } from "./localServerDiscovery";

type AiStatus = { kind: "success" | "error"; message: string };
type ConnectionState = { state: "unknown" | "connected" | "error" };

const UNKNOWN_CONNECTION: ConnectionState = { state: "unknown" };
const PROVIDERS: AiProvider[] = ["local", "openai", "anthropic", "gemini"];

function ConnectionBadge({ status }: { status: ConnectionState }) {
  if (status.state === "unknown") return null;
  if (status.state === "connected") {
    return <span className="settings-modal__connection settings-modal__connection--ok">✓ Connected</span>;
  }
  return <span className="settings-modal__connection settings-modal__connection--error">✗ Not Connected</span>;
}

export function AiSettingsPage() {
  const [aiSettings, setLocalAiSettings] = useState<AiSettings | null>(null);
  const [aiModels, setAiModels] = useState<string[]>([]);
  const [aiStatus, setAiStatus] = useState<AiStatus | null>(null);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [saving, setSaving] = useState(false);
  const [chatStatus, setChatStatus] = useState<ConnectionState>(UNKNOWN_CONNECTION);
  const [embeddingStatus, setEmbeddingStatus] = useState<ConnectionState>(UNKNOWN_CONNECTION);
  const [rerankerStatus, setRerankerStatus] = useState<ConnectionState>(UNKNOWN_CONNECTION);
  const [embeddingCandidates, setEmbeddingCandidates] = useState<AiServerProbe[]>([]);
  const [rerankerCandidates, setRerankerCandidates] = useState<AiServerProbe[]>([]);
  const [scanningEmbedding, setScanningEmbedding] = useState(false);
  const [scanningReranker, setScanningReranker] = useState(false);
  const saveAiSettings = useInteractiveModeStore((state) => state.saveSettings);

  useEffect(() => {
    void getAiSettings()
      .then((settings) => {
        setLocalAiSettings(settings);
        void checkConnections(settings);
      })
      .catch((error) => setAiStatus({ kind: "error", message: String(error) }));
    // Runs once on mount, checking whatever was already saved — a deliberate
    // exception to the exhaustive-deps rule, since re-running this every
    // time the user edits a field would spam connection checks while typing.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const checkConnections = async (aiSettings: AiSettings) => {
    setFetchingModels(true);
    setAiStatus(null);
    try {
      const active = activeProviderConfig(aiSettings);
      let chatModels: string[] | null = null;
      try {
        chatModels = await listAiModels(aiSettings.provider, active.baseUrl, active.apiKey);
      } catch {
        chatModels = null;
      }
      setChatStatus(chatModels ? { state: "connected" } : { state: "error" });
      setAiModels(chatModels ?? []);

      // Embedding/reranking always run against the locally-configured server
      // regardless of which chat provider is active — RAG stays local-only.
      // A separately configured embedding/reranker server (e.g. Nomic or BGE
      // each on their own llama.cpp instance) won't list their model under
      // the local server's /models endpoint — check both configured servers
      // at once and attribute each result back to its own section, so each
      // gets its own connection status and the model dropdowns can show a
      // model that only a non-chat server knows about. Unlike the "Scan for
      // local servers" buttons (which probe several unknown candidate ports
      // and must fail fast), these are servers the user already told us
      // about, so this uses the same patient, no-timeout request the app has
      // always used for a known-good endpoint — an already-loaded model can
      // legitimately take longer than a scan's short timeout to answer.
      const embeddingUrl = aiSettings.embeddingBaseUrl?.trim() || null;
      const rerankerUrl = aiSettings.rerankerEnabled ? (aiSettings.rerankerBaseUrl?.trim() || aiSettings.local.baseUrl.trim()) : null;
      const localUrls = Array.from(new Set([embeddingUrl, rerankerUrl].filter((url): url is string => !!url)));

      const settled = await Promise.allSettled(localUrls.map((url) => listAiModels("local", url, aiSettings.local.apiKey)));
      const modelsByUrl = new Map(localUrls.map((url, i) => {
        const result = settled[i];
        return [url, result.status === "fulfilled" ? result.value : null] as const;
      }));

      // Embedding-based retrieval has no separate on/off UI — it's driven
      // entirely by whether the embedding server is reachable. Connected ->
      // embeddingModel is set to whatever that server serves (enabling the
      // embedding retrieval path in the backend); not configured or not
      // reachable -> embeddingModel is cleared (falls back to plain FTS).
      const embeddingModels = embeddingUrl ? modelsByUrl.get(embeddingUrl) ?? null : null;
      if (embeddingUrl) {
        setEmbeddingStatus(embeddingModels ? { state: "connected" } : { state: "error" });
      } else {
        setEmbeddingStatus(UNKNOWN_CONNECTION);
      }
      setLocalAiSettings((current) => current && { ...current, embeddingModel: embeddingModels?.[0] ?? null });

      if (rerankerUrl) {
        const rerankerModels = modelsByUrl.get(rerankerUrl) ?? null;
        setRerankerStatus(rerankerModels ? { state: "connected" } : { state: "error" });
      } else {
        setRerankerStatus(UNKNOWN_CONNECTION);
      }

      if (!chatModels?.length) setAiStatus({ kind: "error", message: "No configured server responded." });
    } catch (error) {
      setAiStatus({ kind: "error", message: String(error) });
    } finally {
      setFetchingModels(false);
    }
  };

  const handleFetchModels = () => aiSettings && checkConnections(aiSettings);

  const handleProviderChange = (provider: AiProvider) => {
    if (!aiSettings) return;
    setLocalAiSettings({ ...aiSettings, provider });
    setAiModels([]);
    setChatStatus(UNKNOWN_CONNECTION);
    setAiStatus(null);
  };

  const updateActiveConfig = (patch: Partial<ProviderConfig>) => {
    if (!aiSettings) return;
    setLocalAiSettings(withActiveProviderConfig(aiSettings, { ...activeProviderConfig(aiSettings), ...patch }));
  };

  const handleScanEmbedding = async () => {
    if (!aiSettings) return;
    setScanningEmbedding(true);
    setAiStatus(null);
    try {
      const results = await probeAiEndpoints(candidateLocalEndpoints(), aiSettings.local.apiKey);
      setEmbeddingCandidates(results);
      if (!results.length) setAiStatus({ kind: "error", message: "No local servers found on common ports." });
    } catch (error) {
      setAiStatus({ kind: "error", message: String(error) });
    } finally {
      setScanningEmbedding(false);
    }
  };

  const handleScanReranker = async () => {
    if (!aiSettings) return;
    setScanningReranker(true);
    setAiStatus(null);
    try {
      const results = await probeAiEndpoints(candidateLocalEndpoints(), aiSettings.local.apiKey);
      setRerankerCandidates(results);
      if (!results.length) setAiStatus({ kind: "error", message: "No local servers found on common ports." });
    } catch (error) {
      setAiStatus({ kind: "error", message: String(error) });
    } finally {
      setScanningReranker(false);
    }
  };

  const handleSelectEmbeddingCandidate = (baseUrl: string) => {
    if (!aiSettings) return;
    const probe = findProbe(embeddingCandidates, baseUrl);
    if (!probe) return;
    setLocalAiSettings({ ...aiSettings, embeddingBaseUrl: probe.baseUrl, embeddingModel: probe.models[0] ?? null });
    setEmbeddingStatus({ state: "connected" });
  };

  const handleSelectRerankerCandidate = (baseUrl: string) => {
    if (!aiSettings) return;
    const probe = findProbe(rerankerCandidates, baseUrl);
    if (!probe) return;
    setLocalAiSettings({ ...aiSettings, rerankerBaseUrl: probe.baseUrl, rerankerModel: probe.models[0] ?? aiSettings.rerankerModel });
    setRerankerStatus({ state: "connected" });
  };

  const handleSave = async () => {
    if (!aiSettings || saving) return;

    setSaving(true);
    setAiStatus(null);
    try {
      await saveAiSettings(aiSettings);
      setLocalAiSettings(aiSettings);
      setAiStatus({ kind: "success", message: "AI server configuration saved." });
    } catch (error) {
      setAiStatus({ kind: "error", message: String(error) });
    } finally { setSaving(false); }
  };

  return (
    <>
    <section className="settings-modal__page" aria-labelledby="settings-ai-heading">
      <div className="settings-modal__section">
        <h2 id="settings-ai-heading" className="settings-modal__page-heading">AI Settings</h2>
        <h3 className="settings-modal__section-title">AI server <ConnectionBadge status={chatStatus} /></h3>
        {aiSettings && (
          <div className="settings-modal__ai-fields">
            <label className="settings-modal__stacked-field">
              Provider
              <select value={aiSettings.provider} onChange={(e) => handleProviderChange(e.target.value as AiProvider)}>
                {PROVIDERS.map((provider) => (
                  <option key={provider} value={provider}>{PROVIDER_LABELS[provider]}</option>
                ))}
              </select>
            </label>
            {aiSettings.provider === "local" && (
              <label className="settings-modal__stacked-field">
                Server endpoint
                <input
                  value={aiSettings.local.baseUrl}
                  onChange={(e) => updateActiveConfig({ baseUrl: e.target.value })}
                  placeholder="http://localhost:11434/v1"
                />
              </label>
            )}
            <label className="settings-modal__stacked-field">
              API key {aiSettings.provider === "local" && <span>(optional)</span>}
              <input
                type="password"
                value={activeProviderConfig(aiSettings).apiKey ?? ""}
                onChange={(e) => updateActiveConfig({ apiKey: e.target.value || null })}
                autoComplete="off"
              />
            </label>
            <div className="settings-modal__model-row">
              <button
                className="settings-modal__item"
                disabled={fetchingModels || (aiSettings.provider === "local" && !aiSettings.local.baseUrl.trim())}
                onClick={handleFetchModels}
              >
                {fetchingModels ? "Fetching models…" : "Fetch Models"}
              </button>
            </div>
            <label className="settings-modal__stacked-field">
              Default chat model
              <select
                value={activeProviderConfig(aiSettings).chatModel}
                onChange={(e) => updateActiveConfig({ chatModel: e.target.value })}
              >
                {!aiModels.includes(activeProviderConfig(aiSettings).chatModel) && activeProviderConfig(aiSettings).chatModel && (
                  <option value={activeProviderConfig(aiSettings).chatModel}>{activeProviderConfig(aiSettings).chatModel}</option>
                )}
                <option value="">Select a model…</option>
                {aiModels.map((model) => <option key={model} value={model}>{model}</option>)}
              </select>
            </label>
            <h3 className="settings-modal__section-title">Retrieval <ConnectionBadge status={embeddingStatus} /></h3>
            <p className="settings-modal__hint">
              {aiSettings.embeddingModel
                ? `Embedding-based retrieval is enabled, using "${aiSettings.embeddingModel}".`
                : "Embedding-based retrieval is off — connect an embedding server below to enable it. Until then, retrieval uses plain local text search."}
            </p>
            <label className="settings-modal__stacked-field">
              Embedding server endpoint <span>(optional — defaults to the AI server above)</span>
              <div className="settings-modal__endpoint-row">
                <input
                  value={aiSettings.embeddingBaseUrl ?? ""}
                  onChange={(e) => setLocalAiSettings({ ...aiSettings, embeddingBaseUrl: e.target.value || null })}
                  placeholder="http://127.0.0.1:9932/v1"
                />
                <button type="button" className="settings-modal__item" disabled={scanningEmbedding} onClick={handleScanEmbedding}>
                  {scanningEmbedding ? "Scanning…" : "Scan for local servers"}
                </button>
              </div>
            </label>
            {embeddingCandidates.length > 0 && (
              <label className="settings-modal__stacked-field">
                Discovered servers
                <select value="" onChange={(e) => e.target.value && handleSelectEmbeddingCandidate(e.target.value)}>
                  <option value="">Select a discovered server…</option>
                  {embeddingCandidates.map((probe) => (
                    <option key={probe.baseUrl} value={probe.baseUrl}>{formatProbeOption(probe)}</option>
                  ))}
                </select>
              </label>
            )}
            <label className="settings-modal__stacked-field">
              Document embedding prefix
              <input
                value={aiSettings.embeddingDocumentPrefix}
                onChange={(e) => setLocalAiSettings({ ...aiSettings, embeddingDocumentPrefix: e.target.value })}
              />
            </label>
            <label className="settings-modal__stacked-field">
              Query embedding prefix
              <input
                value={aiSettings.embeddingQueryPrefix}
                onChange={(e) => setLocalAiSettings({ ...aiSettings, embeddingQueryPrefix: e.target.value })}
              />
            </label>
            <label className="settings-modal__stacked-field">
              Candidate chunks considered
              <input
                type="number" min={1} max={200}
                value={aiSettings.retrievalCandidateCount}
                onChange={(e) => setLocalAiSettings({ ...aiSettings, retrievalCandidateCount: Number(e.target.value) || 1 })}
              />
            </label>
            <label className="settings-modal__stacked-field">
              Final chunks sent to the model
              <input
                type="number" min={1} max={50}
                value={aiSettings.retrievalFinalCount}
                onChange={(e) => setLocalAiSettings({ ...aiSettings, retrievalFinalCount: Number(e.target.value) || 1 })}
              />
            </label>
            <h3 className="settings-modal__section-title">Reranking <ConnectionBadge status={rerankerStatus} /></h3>
            <label className="settings-modal__stacked-field settings-modal__stacked-field--checkbox">
              <input
                type="checkbox"
                checked={aiSettings.rerankerEnabled}
                onChange={(e) => setLocalAiSettings({ ...aiSettings, rerankerEnabled: e.target.checked })}
              />
              Enable reranking
            </label>
            <label className="settings-modal__stacked-field">
              Reranker server endpoint <span>(optional — defaults to the AI server above)</span>
              <div className="settings-modal__endpoint-row">
                <input
                  value={aiSettings.rerankerBaseUrl ?? ""}
                  onChange={(e) => setLocalAiSettings({ ...aiSettings, rerankerBaseUrl: e.target.value || null })}
                  placeholder="http://127.0.0.1:9933"
                  disabled={!aiSettings.rerankerEnabled}
                />
                <button type="button" className="settings-modal__item" disabled={scanningReranker || !aiSettings.rerankerEnabled} onClick={handleScanReranker}>
                  {scanningReranker ? "Scanning…" : "Scan for local servers"}
                </button>
              </div>
            </label>
            {rerankerCandidates.length > 0 && (
              <label className="settings-modal__stacked-field">
                Discovered servers
                <select value="" onChange={(e) => e.target.value && handleSelectRerankerCandidate(e.target.value)} disabled={!aiSettings.rerankerEnabled}>
                  <option value="">Select a discovered server…</option>
                  {rerankerCandidates.map((probe) => (
                    <option key={probe.baseUrl} value={probe.baseUrl}>{formatProbeOption(probe)}</option>
                  ))}
                </select>
              </label>
            )}
            <label className="settings-modal__stacked-field">
              Reranker model
              <input
                value={aiSettings.rerankerModel ?? ""}
                onChange={(e) => setLocalAiSettings({ ...aiSettings, rerankerModel: e.target.value || null })}
                placeholder="bge-reranker-v2-m3"
                disabled={!aiSettings.rerankerEnabled}
              />
            </label>
            <button
              className="settings-modal__item"
              disabled={saving || (aiSettings.provider === "local" && !aiSettings.local.baseUrl.trim()) || !activeProviderConfig(aiSettings).chatModel.trim()}
              onClick={handleSave}
            >
              {saving ? "Saving…" : "Save AI Configuration"}
            </button>
          </div>
        )}
        {aiStatus && (
          <p role={aiStatus.kind === "error" ? "alert" : "status"} className={`settings-modal__status${aiStatus.kind === "error" ? " settings-modal__status--error" : ""}`}>
            {aiStatus.message}
          </p>
        )}
      </div>
    </section>
    </>
  );
}
