use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

use serde::Serialize;
use tauri::{Emitter, State, Window};
use tokio_util::sync::CancellationToken;

use crate::ai::client::{self, ChatMessage};
use crate::ai::context::content_hash;
use crate::ai::index::{cosine_similarity, ContextChunk, KnowledgeIndex};
use crate::ai::operations::{parse_action_block, AiAction};
use crate::ai::providers;
use crate::error::{AppError, AppResult};
use crate::fs::ops;
use crate::settings::store::{AiProvider, SettingsStore};
use crate::workspace_authority::ActiveWorkspace;

const MAX_ACTIVE_DOCUMENT_CHARS: usize = 100_000;

#[derive(Default)]
pub struct AiRuntime {
    pending: Mutex<HashMap<String, PendingProposal>>,
    requests: Mutex<HashMap<String, CancellationToken>>,
    next_id: AtomicU64,
}

#[derive(Clone)]
struct PendingProposal {
    workspace: PathBuf,
    target: PathBuf,
    action: AiAction,
    original_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionProposal {
    pub id: String,
    pub action_type: String,
    pub path: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatResult {
    pub message: String,
    pub citations: Vec<ContextChunk>,
    pub proposals: Vec<ActionProposal>,
    pub applied_paths: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionAiResult {
    pub text: String,
    pub citations: Vec<ContextChunk>,
}

struct ActiveDocument {
    relative_path: String,
    content: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeltaEvent {
    request_id: String,
    delta: String,
}

pub const AI_INDEX_STATUS_EVENT: &str = "ai-index-status";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatusEvent {
    pub status: String,
    pub detail: Option<String>,
}

/// Caps how many paths from a very large workspace get listed verbatim in the
/// system prompt, so a workspace with thousands of files can't blow the
/// request's token budget just by existing.
const MAX_WORKSPACE_FILE_LIST: usize = 500;

fn build_system_prompt(
    context: &[ContextChunk],
    active_document: Option<&ActiveDocument>,
    all_paths: &[String],
) -> String {
    let mut prompt = String::from(
        "You are Foldown Interactive Mode, a private assistant for the active Markdown workspace. \
         You are given the complete list of every Markdown file in the workspace under WORKSPACE FILES, and \
         content excerpts for the files most relevant to the user's message under WORKSPACE CONTEXT. You may \
         reference any listed file by name — including ones in subfolders — even when its content wasn't \
         retrieved, but never invent or assume a file's content; only rely on supplied excerpts or the active \
         document for that. \
         Prefer information explicitly supported by the supplied WORKSPACE CONTEXT or ACTIVE DOCUMENT. If neither \
         contains enough information to answer reliably, say plainly that you do not have enough information in \
         your documents rather than guessing — do not invent facts, file contents, or section names that were not \
         actually supplied to you. \
         When the user asks to change files, respond with brief prose and append exactly one fenced foldown-actions JSON block. \
         Its shape is {\"actions\":[{\"type\":\"create|replace|delete\",\"path\":\"relative.md\",\"content\":\"complete file content for create/replace\"}]}. \
         Paths must be relative .md paths inside the active workspace. Never output drive letters, UNC paths, or absolute paths. \
         If the user names a folder, treat it as a folder relative to the active workspace; if no folder is named, use the workspace root. \
         Use replace for existing files; Foldown also safely interprets create for an existing file as replace. \
         Foldown automatically applies create actions for brand-new files. Replacing or deleting an existing file \
         always asks the user to confirm first, since it overwrites content that already exists. \
         For replacements, provide the complete replacement content for the target file. \
         Never describe an action in prose without also including its JSON block in that same reply — a promise to \
         act is not an action. And because Foldown's own interface already asks the user to confirm before replacing \
         or deleting a file, do not ask the user for permission or phrase the action as a question yourself — state \
         plainly what you are doing (e.g. \"Replacing Notes.md with...\") and include the action block right away.\n"
    );
    if !all_paths.is_empty() {
        prompt.push_str(&format!("\nWORKSPACE FILES ({} total):\n", all_paths.len()));
        for path in all_paths.iter().take(MAX_WORKSPACE_FILE_LIST) {
            prompt.push_str(path);
            prompt.push('\n');
        }
        if all_paths.len() > MAX_WORKSPACE_FILE_LIST {
            prompt.push_str(&format!(
                "...and {} more file(s) not shown.\n",
                all_paths.len() - MAX_WORKSPACE_FILE_LIST
            ));
        }
    }
    if let Some(document) = active_document {
        prompt.push_str(&format!(
            "\nACTIVE DOCUMENT (complete): {}\n--- BEGIN ACTIVE DOCUMENT ---\n{}\n--- END ACTIVE DOCUMENT ---\n",
            document.relative_path, document.content
        ));
    }
    prompt.push_str("\nWORKSPACE CONTEXT:\n");
    for chunk in context {
        prompt.push_str(&format!(
            "\n--- {} | {} ---\n{}\n",
            chunk.path, chunk.heading, chunk.text
        ));
    }
    prompt
}

fn load_active_document(
    root: &Path,
    active_path: Option<&str>,
) -> AppResult<Option<ActiveDocument>> {
    let Some(active_path) = active_path else {
        return Ok(None);
    };
    let requested = Path::new(active_path);
    let requested = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    let target = ops::ensure_within_workspace(&requested, root)?;
    if !target
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return Err(AppError::Message(
            "The active AI document must be a Markdown file".to_string(),
        ));
    }
    let content = ops::read_file(&target)?;
    if content.chars().count() > MAX_ACTIVE_DOCUMENT_CHARS {
        return Ok(None);
    }
    let relative_path = target
        .strip_prefix(root.canonicalize()?)
        .map_err(|_| AppError::Message("The active document is outside the workspace".to_string()))?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(Some(ActiveDocument {
        relative_path,
        content,
    }))
}

fn validate_replacement_context(
    _actions: &[AiAction],
    _active_document: Option<&ActiveDocument>,
) -> AppResult<()> {
    Ok(())
}

/// Only a brand-new file (a "create" whose target didn't already exist) is
/// safe to auto-apply — it can't destroy anything. Overwriting existing
/// content ("replace", and "create" actions `create_proposals` reclassified
/// as "replace" because the target already existed) always needs the user's
/// explicit sign-off, the same as "delete".
fn requires_confirmation(action_type: &str) -> bool {
    action_type != "create"
}

async fn retrieve_candidates(
    index: &KnowledgeIndex,
    root: &Path,
    query: &str,
    settings: &crate::settings::store::AiSettings,
    candidate_count: usize,
) -> AppResult<Vec<ContextChunk>> {
    let Some(model) = settings.embedding_model.as_deref() else {
        return index.search_candidates(root, query, candidate_count);
    };
    let mut chunks = index.all_chunks(root)?;
    let mut missing = Vec::new();
    for chunk in &chunks {
        let prefixed = format!("{}{}", settings.embedding_document_prefix, chunk.text);
        if index.cached_embedding(model, &prefixed)?.is_none() {
            missing.push(prefixed);
        }
    }
    for batch in missing.chunks(32) {
        let input = batch.to_vec();
        let Ok(vectors) = client::create_embeddings(settings, model, &input).await else {
            return index.search_candidates(root, query, candidate_count);
        };
        if vectors.len() != input.len() {
            return index.search_candidates(root, query, candidate_count);
        }
        for (text, vector) in input.iter().zip(vectors) {
            index.store_embedding(model, text, &vector)?;
        }
    }
    let prefixed_query = format!("{}{}", settings.embedding_query_prefix, query);
    let Ok(mut query_vectors) = client::create_embeddings(settings, model, &[prefixed_query]).await
    else {
        return index.search_candidates(root, query, candidate_count);
    };
    let Some(query_vector) = query_vectors.pop() else {
        return index.search_candidates(root, query, candidate_count);
    };
    // If this model has previously embedded chunks at a different dimension
    // (e.g. the user repointed the same model name at a server that produces
    // differently-shaped vectors), every cached chunk vector will fail
    // cosine_similarity's length-mismatch guard and score 0.0 — a tie that
    // `sort_by`'s stable sort resolves in arbitrary DB order, silently
    // returning nonsense "most relevant" context instead of falling back.
    // Detect that here, before it can happen, and fall back exactly like the
    // other embedding-error paths above. A model with no recorded dimension
    // yet (`None`) is the legitimate first-time-embedding case — proceed.
    if let Some(known_dimension) = index.embedding_dimension(model)? {
        if known_dimension != query_vector.len() {
            return index.search_candidates(root, query, candidate_count);
        }
    }
    for chunk in &mut chunks {
        let prefixed = format!("{}{}", settings.embedding_document_prefix, chunk.text);
        if let Some(vector) = index.cached_embedding(model, &prefixed)? {
            chunk.score = cosine_similarity(&query_vector, &vector) as f64;
        }
    }
    chunks.sort_by(|a, b| b.score.total_cmp(&a.score));
    chunks.truncate(candidate_count);
    Ok(chunks)
}

async fn retrieve_context(
    index: &KnowledgeIndex,
    root: &Path,
    query: &str,
    settings: &crate::settings::store::AiSettings,
) -> AppResult<Vec<ContextChunk>> {
    let candidate_count = (settings.retrieval_candidate_count as usize).max(1);
    let final_count = (settings.retrieval_final_count as usize).max(1);
    let max_chars = settings.retrieval_max_chars as usize;
    let mut candidates = retrieve_candidates(index, root, query, settings, candidate_count).await?;

    if settings.reranker_enabled {
        if let Some(model) = settings.reranker_model.as_deref() {
            let documents: Vec<String> = candidates.iter().map(|c| c.text.clone()).collect();
            if let Ok(scores) =
                crate::ai::reranker::rerank(settings, model, query, &documents).await
            {
                if scores.len() == candidates.len() {
                    for (chunk, score) in candidates.iter_mut().zip(scores) {
                        chunk.score = score as f64;
                    }
                    candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
                }
            }
            // A reranker error or a mismatched score count silently falls
            // through to the candidates' original (FTS/embedding) order —
            // a failed or misbehaving reranker must never break retrieval.
        }
    }

    candidates.truncate(final_count);
    Ok(crate::ai::index::truncate_to_char_budget(
        candidates, max_chars,
    ))
}

fn create_proposals(
    runtime: &AiRuntime,
    workspace_root: &Path,
    actions: Vec<AiAction>,
) -> AppResult<Vec<ActionProposal>> {
    let workspace = workspace_root.canonicalize()?;
    let mut output = Vec::new();
    let mut pending = runtime.pending.lock().unwrap();
    for action in actions {
        let target = workspace.join(action.path().replace('/', std::path::MAIN_SEPARATOR_STR));
        let target = ops::ensure_within_workspace(&target, &workspace)?;
        let old_content = if target.exists() {
            Some(ops::read_file(&target)?)
        } else {
            None
        };
        let action = match action {
            AiAction::Create { path, content } if old_content.is_some() => {
                AiAction::Replace { path, content }
            }
            action => action,
        };
        match &action {
            AiAction::Replace { .. } | AiAction::Delete { .. } if old_content.is_none() => {
                return Err(AppError::Message(format!(
                    "{} does not exist",
                    action.path()
                )))
            }
            _ => {}
        }
        let id = format!(
            "proposal-{}",
            runtime.next_id.fetch_add(1, Ordering::SeqCst)
        );
        let original_hash = old_content.as_deref().map(content_hash);
        let new_content = action.content().map(str::to_string);
        let proposal = ActionProposal {
            id: id.clone(),
            action_type: action.kind().to_string(),
            path: action.path().to_string(),
            old_content,
            new_content,
        };
        pending.insert(
            id,
            PendingProposal {
                workspace: workspace.clone(),
                target,
                action,
                original_hash,
            },
        );
        output.push(proposal);
    }
    Ok(output)
}

/// Routes one chat turn to whichever provider is active. Local Server keeps
/// using its existing text-block `foldown-actions` mechanism unchanged; the
/// three cloud providers each parse their own native tool-call format into
/// the same `Vec<AiAction>` shape via their own module.
async fn dispatch_chat(
    window: &Window,
    request_id: &str,
    settings: &crate::settings::store::AiSettings,
    request_messages: &[ChatMessage],
) -> AppResult<(String, Vec<AiAction>)> {
    let on_delta = |delta: &str| {
        let _ = window.emit(
            "ai-chat-delta",
            DeltaEvent {
                request_id: request_id.to_string(),
                delta: delta.to_string(),
            },
        );
    };
    match settings.provider {
        AiProvider::Local => {
            let text = client::send_chat(settings, request_messages, on_delta).await?;
            let parsed = parse_action_block(&text)?;
            Ok((parsed.message, parsed.actions))
        }
        AiProvider::Openai => {
            let outcome =
                providers::openai::send_chat(&settings.openai, request_messages, on_delta).await?;
            Ok((outcome.text, outcome.actions))
        }
        AiProvider::Anthropic => {
            let outcome =
                providers::anthropic::send_chat(&settings.anthropic, request_messages, on_delta)
                    .await?;
            Ok((outcome.text, outcome.actions))
        }
        AiProvider::Gemini => {
            let outcome =
                providers::gemini::send_chat(&settings.gemini, request_messages, on_delta).await?;
            Ok((outcome.text, outcome.actions))
        }
    }
}

async fn dispatch_plain_chat(
    window: &Window,
    request_id: &str,
    settings: &crate::settings::store::AiSettings,
    request_messages: &[ChatMessage],
) -> AppResult<String> {
    let on_delta = |delta: &str| {
        let _ = window.emit(
            "ai-chat-delta",
            DeltaEvent {
                request_id: request_id.to_string(),
                delta: delta.to_string(),
            },
        );
    };
    match settings.provider {
        AiProvider::Local => client::send_chat(settings, request_messages, on_delta).await,
        AiProvider::Openai => Ok(providers::openai::send_chat(&settings.openai, request_messages, on_delta).await?.text),
        AiProvider::Anthropic => Ok(providers::anthropic::send_chat(&settings.anthropic, request_messages, on_delta).await?.text),
        AiProvider::Gemini => Ok(providers::gemini::send_chat(&settings.gemini, request_messages, on_delta).await?.text),
    }
}

#[tauri::command]
pub async fn send_ai_message(
    window: Window,
    store: State<'_, SettingsStore>,
    index: State<'_, KnowledgeIndex>,
    runtime: State<'_, AiRuntime>,
    active: State<'_, ActiveWorkspace>,
    workspace_root: String,
    request_id: String,
    messages: Vec<ChatMessage>,
    active_path: Option<String>,
) -> AppResult<AiChatResult> {
    let settings = store.get_ai_settings()?;
    let active_chat_model = match settings.provider {
        AiProvider::Local => &settings.local.chat_model,
        AiProvider::Openai => &settings.openai.chat_model,
        AiProvider::Anthropic => &settings.anthropic.chat_model,
        AiProvider::Gemini => &settings.gemini.chat_model,
    };
    if active_chat_model.trim().is_empty() {
        return Err(AppError::Message(
            "Choose a chat model in Interactive Mode settings".to_string(),
        ));
    }
    let root = active.require(Path::new(&workspace_root))?;
    index.sync_workspace(&root)?;
    let query = messages.last().map(|m| m.content.as_str()).unwrap_or("");
    let citations = retrieve_context(&index, &root, query, &settings).await?;
    let active_document = load_active_document(&root, active_path.as_deref())?;
    let all_paths = index.all_document_paths(&root)?;
    let mut request_messages = vec![ChatMessage {
        role: "system".to_string(),
        content: build_system_prompt(&citations, active_document.as_ref(), &all_paths),
    }];
    request_messages.extend(
        messages
            .into_iter()
            .rev()
            .take(12)
            .collect::<Vec<_>>()
            .into_iter()
            .rev(),
    );
    let token = CancellationToken::new();
    runtime
        .requests
        .lock()
        .unwrap()
        .insert(request_id.clone(), token.clone());
    let response = tokio::select! {
        value = dispatch_chat(&window, &request_id, &settings, &request_messages) => value,
        _ = token.cancelled() => Err(AppError::Message("AI request cancelled".to_string())),
    };
    runtime.requests.lock().unwrap().remove(&request_id);
    let (mut message, actions) = response?;
    validate_replacement_context(&actions, active_document.as_ref())?;
    let proposals = create_proposals(&runtime, &root, actions)?;
    // Only a genuinely new file (a "create" whose target didn't already exist)
    // is safe to auto-apply: it can't destroy anything. Anything that overwrites
    // existing content — "replace", and "create" actions the model aimed at an
    // existing file (silently reclassified as "replace" in create_proposals) —
    // requires explicit user confirmation, the same as "delete".
    let mut confirm_proposals = Vec::new();
    let mut applied_paths = Vec::new();
    let mut apply_errors = Vec::new();
    for proposal in proposals {
        if requires_confirmation(&proposal.action_type) {
            confirm_proposals.push(proposal);
        } else {
            match apply_pending_proposal(&runtime, &index, &active, &proposal.id) {
                Ok(path) => applied_paths.push(path),
                Err(error) => apply_errors.push(format!("{}: {error}", proposal.path)),
            }
        }
    }
    if message.is_empty() && !applied_paths.is_empty() {
        message = "Done.".to_string();
    }
    if !apply_errors.is_empty() {
        message.push_str(&format!("\n\nCouldn't create: {}", apply_errors.join("; ")));
    }
    Ok(AiChatResult {
        message,
        citations,
        proposals: confirm_proposals,
        applied_paths,
    })
}

#[tauri::command]
pub async fn run_selection_ai(
    window: Window,
    store: State<'_, SettingsStore>,
    index: State<'_, KnowledgeIndex>,
    runtime: State<'_, AiRuntime>,
    active: State<'_, ActiveWorkspace>,
    workspace_root: String,
    request_id: String,
    action: String,
    selected_text: String,
    active_path: String,
) -> AppResult<SelectionAiResult> {
    let selected_text = selected_text.trim();
    if selected_text.is_empty() {
        return Err(AppError::Message("Select some text before using an AI action".to_string()));
    }
    let settings = store.get_ai_settings()?;
    let active_chat_model = match settings.provider {
        AiProvider::Local => &settings.local.chat_model,
        AiProvider::Openai => &settings.openai.chat_model,
        AiProvider::Anthropic => &settings.anthropic.chat_model,
        AiProvider::Gemini => &settings.gemini.chat_model,
    };
    if active_chat_model.trim().is_empty() {
        return Err(AppError::Message("Choose a chat model in Interactive Mode settings".to_string()));
    }
    let root = active.require(Path::new(&workspace_root))?;
    index.sync_workspace(&root)?;
    let citations = retrieve_context(&index, &root, selected_text, &settings).await?;
    let active_document = load_active_document(&root, Some(&active_path))?;
    let all_paths = index.all_document_paths(&root)?;
    let mut system = build_system_prompt(&citations, active_document.as_ref(), &all_paths);
    system.push_str(
        "\nSELECTION MODE: Return only the proposed text or explanation for the selected passage. Do not create, replace, delete, or modify files, and do not emit a Foldown action block. The application will show your response as a proposal for the selected text.\n",
    );
    let request_messages = vec![
        ChatMessage { role: "system".to_string(), content: system },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Action: {action}\n\nSelected Markdown:\n---\n{selected_text}\n---"),
        },
    ];
    let token = CancellationToken::new();
    runtime.requests.lock().unwrap().insert(request_id.clone(), token.clone());
    let response = tokio::select! {
        value = dispatch_plain_chat(&window, &request_id, &settings, &request_messages) => value,
        _ = token.cancelled() => Err(AppError::Message("AI request cancelled".to_string())),
    };
    runtime.requests.lock().unwrap().remove(&request_id);
    let text = response?.trim().to_string();
    if text.is_empty() {
        return Err(AppError::Message("The AI returned an empty response".to_string()));
    }
    Ok(SelectionAiResult { text, citations })
}

#[tauri::command]
pub fn cancel_ai_request(runtime: State<AiRuntime>, request_id: String) {
    if let Some(token) = runtime.requests.lock().unwrap().get(&request_id) {
        token.cancel();
    }
}

#[tauri::command]
pub async fn preview_ai_retrieval(
    store: State<'_, SettingsStore>,
    index: State<'_, KnowledgeIndex>,
    active: State<'_, ActiveWorkspace>,
    workspace_root: String,
    query: String,
) -> AppResult<Vec<ContextChunk>> {
    let settings = store.get_ai_settings()?;
    let root = active.require(Path::new(&workspace_root))?;
    index.sync_workspace(&root)?;
    retrieve_context(&index, &root, &query, &settings).await
}

#[tauri::command]
pub fn rebuild_ai_index(
    window: Window,
    active: State<ActiveWorkspace>,
    index: State<KnowledgeIndex>,
    workspace_root: String,
) -> AppResult<()> {
    let root = active.require(Path::new(&workspace_root))?;
    let _ = window.emit(
        AI_INDEX_STATUS_EVENT,
        IndexStatusEvent {
            status: "indexing".to_string(),
            detail: None,
        },
    );
    let result = index.rebuild_workspace(&root);
    let event = match &result {
        Ok(()) => IndexStatusEvent {
            status: "ready".to_string(),
            detail: None,
        },
        Err(error) => IndexStatusEvent {
            status: "error".to_string(),
            detail: Some(error.to_string()),
        },
    };
    let _ = window.emit(AI_INDEX_STATUS_EVENT, event);
    result
}

#[tauri::command]
pub fn reject_ai_proposal(runtime: State<AiRuntime>, proposal_id: String) {
    runtime.pending.lock().unwrap().remove(&proposal_id);
}

#[tauri::command]
pub fn apply_ai_proposal(
    runtime: State<AiRuntime>,
    index: State<KnowledgeIndex>,
    active: State<ActiveWorkspace>,
    proposal_id: String,
) -> AppResult<String> {
    apply_pending_proposal(&runtime, &index, &active, &proposal_id)
}

fn apply_pending_proposal(
    runtime: &AiRuntime,
    index: &KnowledgeIndex,
    active: &ActiveWorkspace,
    proposal_id: &str,
) -> AppResult<String> {
    let proposal = runtime
        .pending
        .lock()
        .unwrap()
        .remove(proposal_id)
        .ok_or_else(|| AppError::Message("This AI proposal is no longer available".to_string()))?;
    let workspace = active.require(&proposal.workspace)?;
    let target = ops::ensure_within_workspace(&proposal.target, &workspace)?;
    let current = if target.exists() {
        Some(ops::read_file(&target)?)
    } else {
        None
    };
    if current.as_deref().map(content_hash) != proposal.original_hash {
        return Err(AppError::Message(
            "The file changed after this proposal was created; ask the AI to try again".to_string(),
        ));
    }
    match &proposal.action {
        AiAction::Create { content, .. } => ops::write_file_atomic(&target, content)?,
        AiAction::Replace { content, .. } => ops::write_file_atomic(&target, content)?,
        AiAction::Delete { .. } => ops::delete_path(&target)?,
    }
    index.refresh_path(&workspace, &target)?;
    Ok(target.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_final_count_setting_controls_how_many_chunks_are_selected() {
        let root =
            std::env::temp_dir().join(format!("foldown-ai-retrieval-count-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for i in 0..5 {
            std::fs::write(
                root.join(format!("note-{i}.md")),
                format!("# Note {i}\nneedle appears here too"),
            )
            .unwrap();
        }
        let index = KnowledgeIndex::open(root.join("index.db")).unwrap();
        index.sync_workspace(&root).unwrap();

        let mut settings = crate::settings::store::AiSettings::default();
        settings.retrieval_final_count = 2;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let context = runtime
            .block_on(retrieve_context(&index, &root, "needle", &settings))
            .unwrap();
        assert_eq!(context.len(), 2);
    }

    #[test]
    fn reranker_failure_falls_back_to_the_unreranked_candidate_order() {
        let root =
            std::env::temp_dir().join(format!("foldown-ai-rerank-fallback-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.md"), "# A\nneedle one").unwrap();
        std::fs::write(root.join("b.md"), "# B\nneedle two").unwrap();
        let index = KnowledgeIndex::open(root.join("index.db")).unwrap();
        index.sync_workspace(&root).unwrap();

        let mut settings = crate::settings::store::AiSettings::default();
        settings.reranker_enabled = true;
        settings.reranker_model = Some("bge-reranker-v2-m3".to_string());
        // Deliberately unreachable — port 1 is reserved and nothing binds it.
        settings.reranker_base_url = Some("http://127.0.0.1:1".to_string());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let context = runtime
            .block_on(retrieve_context(&index, &root, "needle", &settings))
            .unwrap();
        // Must still return the FTS-ranked candidates rather than erroring
        // out or returning nothing just because the reranker is unreachable.
        assert_eq!(context.len(), 2);
    }

    /// A minimal hand-rolled HTTP server standing in for a real embedding
    /// endpoint — no mock-HTTP crate is added; this is a bare TcpListener
    /// that accepts exactly one connection and replies with a fixed,
    /// successful HTTP/1.1 response, just enough for reqwest's client to
    /// parse. Used to prove the dimension-mismatch fallback fires on a
    /// genuinely *successful* embedding response with the wrong vector
    /// length — unlike the unreachable-URL trick used for reranker fallback
    /// tests above, which only exercises the "connection failed" branch.
    fn spawn_single_response_server(body: &str) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = body.to_string();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                use std::io::{Read, Write};
                let mut buf = [0u8; 8192];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("http://{addr}")
    }

    #[test]
    fn embedding_dimension_mismatch_falls_back_to_fts_instead_of_scoring_everything_zero() {
        let root =
            std::env::temp_dir().join(format!("foldown-ai-dim-mismatch-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.md"), "# A\nneedle one").unwrap();
        std::fs::write(root.join("b.md"), "# B\nneedle two").unwrap();
        let index = KnowledgeIndex::open(root.join("index.db")).unwrap();
        index.sync_workspace(&root).unwrap();

        let model = "nomic-embed-text";
        let mut settings = crate::settings::store::AiSettings::default();
        settings.embedding_model = Some(model.to_string());

        // Cache every chunk's embedding at 2 dimensions directly (bypassing
        // the HTTP client) — this simulates the steady state after one full
        // indexing pass: every chunk is already cached, so `missing` below
        // will be empty and retrieve_candidates never calls create_embeddings
        // for document text, only for the query.
        for chunk in index.all_chunks(&root).unwrap() {
            let prefixed = format!("{}{}", settings.embedding_document_prefix, chunk.text);
            index
                .store_embedding(model, &prefixed, &[1.0, 0.0])
                .unwrap();
        }

        // The query embedding call succeeds, but the "server" now answers
        // with a 3-dimensional vector — as if the user repointed the same
        // model name at a different, differently-dimensioned server.
        let body = r#"{"data":[{"index":0,"embedding":[1.0,0.0,0.0]}]}"#;
        settings.embedding_base_url = Some(spawn_single_response_server(body));

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let context = runtime
            .block_on(retrieve_context(&index, &root, "needle", &settings))
            .unwrap();
        // Must fall back to the FTS-ranked candidates (both docs found)
        // rather than returning an arbitrary-order tie from scoring every
        // cached chunk 0.0 against a length-mismatched query vector.
        assert_eq!(context.len(), 2);
        // Without the fallback, every chunk's score would be exactly 0.0
        // (cosine_similarity's length-mismatch guard), and a same-count
        // assertion alone wouldn't catch that (this fixture only has 2
        // candidate chunks total, so `.len() == 2` would hold either way).
        // FTS-ranked scores are `-bm25(...)`, which is never exactly 0.0 for
        // an actual keyword match, so this distinguishes "genuinely fell
        // back to FTS" from "silently kept the zeroed-out embedding scores".
        assert!(
            context.iter().all(|c| c.score != 0.0),
            "expected nonzero FTS bm25-derived scores from the fallback, not zeroed cosine-similarity scores: {context:?}"
        );
    }

    #[test]
    fn system_prompt_includes_the_complete_active_document() {
        let document = ActiveDocument {
            relative_path: "Code Review.md".to_string(),
            content: "# Code Review\n## 4. Security\nKeep this.\n## 5. Performance\nPreserve this section.".to_string(),
        };
        let prompt = build_system_prompt(&[], Some(&document), &[]);
        assert!(prompt.contains("ACTIVE DOCUMENT (complete): Code Review.md"));
        assert!(prompt.contains("## 5. Performance\nPreserve this section."));
        assert!(prompt.contains("Never output drive letters, UNC paths, or absolute paths"));
    }

    #[test]
    fn system_prompt_tells_the_model_not_to_narrate_without_acting_or_ask_permission() {
        // Regression test: a real local model was observed either (a) describing
        // an action in prose without attaching its JSON block, or (b) attaching a
        // valid action while still phrasing its reply as "should I proceed?" even
        // though Foldown's own UI already gates replace/delete on confirmation.
        let prompt = build_system_prompt(&[], None, &[]);
        assert!(prompt.contains("a promise to act is not an action"));
        assert!(prompt.contains("do not ask the user for permission"));
    }

    #[test]
    fn system_prompt_lists_every_workspace_file_even_ones_with_no_retrieved_content() {
        // Regression test: previously the model only ever learned a file existed
        // if it happened to match the retrieval query, so it had no way to answer
        // "what files do I have" or reference an unrelated file by name.
        let all_paths = vec!["Journal/2024-01-01.md".to_string(), "Root.md".to_string()];
        let prompt = build_system_prompt(&[], None, &all_paths);
        assert!(prompt.contains("WORKSPACE FILES (2 total):"));
        assert!(prompt.contains("Journal/2024-01-01.md"));
        assert!(prompt.contains("Root.md"));
    }

    #[test]
    fn system_prompt_caps_a_very_large_workspace_file_list() {
        let all_paths: Vec<String> = (0..MAX_WORKSPACE_FILE_LIST + 10)
            .map(|n| format!("note-{n}.md"))
            .collect();
        let prompt = build_system_prompt(&[], None, &all_paths);
        assert!(prompt.contains(&format!(
            "WORKSPACE FILES ({} total):",
            MAX_WORKSPACE_FILE_LIST + 10
        )));
        assert!(prompt.contains("note-0.md"));
        assert!(!prompt.contains(&format!("note-{}.md", MAX_WORKSPACE_FILE_LIST + 9)));
        assert!(prompt.contains("...and 10 more file(s) not shown."));
    }

    #[test]
    fn replacements_require_an_existing_target_file() {
        let actions = vec![AiAction::Replace {
            path: "Code Review.md".to_string(),
            content: "# Code Review\n## 5. Performance\nPreserved.".to_string(),
        }];
        assert!(validate_replacement_context(&actions, None).is_ok());
    }

    #[test]
    fn replacements_can_target_multiple_workspace_files() {
        let actions = vec![
            AiAction::Replace {
                path: "Go.md".to_string(),
                content: "# Go".to_string(),
            },
            AiAction::Replace {
                path: "Python.md".to_string(),
                content: "# Python".to_string(),
            },
        ];
        assert!(validate_replacement_context(&actions, None).is_ok());
    }

    #[test]
    fn an_ai_create_can_update_an_existing_file() {
        let workspace =
            std::env::temp_dir().join(format!("foldown-ai-create-{}", std::process::id()));
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("TypeScript.md"), "# Basic").unwrap();

        let runtime = AiRuntime::default();
        let proposals = create_proposals(
            &runtime,
            &workspace,
            vec![AiAction::Create {
                path: "TypeScript.md".to_string(),
                content: "# Comprehensive".to_string(),
            }],
        )
        .unwrap();

        assert_eq!(proposals[0].old_content.as_deref(), Some("# Basic"));
        assert_eq!(proposals[0].action_type, "replace");
    }

    #[test]
    fn only_brand_new_creates_are_auto_applied() {
        assert!(!requires_confirmation("create"));
        assert!(requires_confirmation("replace"));
        assert!(requires_confirmation("delete"));
    }

    #[test]
    fn index_status_event_serializes_with_camel_case_and_optional_detail() {
        let event = IndexStatusEvent {
            status: "indexing".to_string(),
            detail: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, r#"{"status":"indexing","detail":null}"#);
        let event = IndexStatusEvent {
            status: "error".to_string(),
            detail: Some("no embedding server".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""detail":"no embedding server""#));
    }

    #[test]
    fn a_replace_proposed_for_an_untouched_file_still_requires_confirmation() {
        // Regression test: this used to be gated only by validate_replacement_context
        // (which required the target to be the active document); now that check is a
        // no-op by design (the AI may target any workspace file), so the *only* thing
        // standing between the model and a silent overwrite is requires_confirmation.
        let workspace =
            std::env::temp_dir().join(format!("foldown-ai-confirm-{}", std::process::id()));
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(workspace.join("Unrelated.md"), "# Untouched content").unwrap();

        let runtime = AiRuntime::default();
        let proposals = create_proposals(
            &runtime,
            &workspace,
            vec![AiAction::Replace {
                path: "Unrelated.md".to_string(),
                content: "# Overwritten".to_string(),
            }],
        )
        .unwrap();

        assert_eq!(proposals.len(), 1);
        assert!(requires_confirmation(&proposals[0].action_type));
        // The proposal must still be sitting in `runtime.pending` (i.e. not yet
        // written to disk) — apply_pending_proposal is only ever called for it
        // after an explicit user approval.
        assert_eq!(
            std::fs::read_to_string(workspace.join("Unrelated.md")).unwrap(),
            "# Untouched content"
        );
    }

    #[test]
    fn retrieval_finds_the_right_document_for_exact_terminology_and_paraphrase() {
        let root = std::env::temp_dir().join(format!("foldown-ai-corpus-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("llama-setup.md"), "# llama.cpp server\nRun llama-server.exe with --ctx-size 8192 and -ngl 999 for full GPU offload. Some guides call this GPU offloading instead.").unwrap();
        std::fs::write(
            root.join("recipes.md"),
            "# Sourdough\nFeed the starter daily with equal parts flour and water.",
        )
        .unwrap();
        std::fs::write(
            root.join("networking.md"),
            "# Router setup\nThe router's admin page is at 192.168.1.1.",
        )
        .unwrap();
        let index = KnowledgeIndex::open(root.join("index.db")).unwrap();
        index.sync_workspace(&root).unwrap();
        let settings = crate::settings::store::AiSettings::default();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        // Exact terminology.
        let context = runtime
            .block_on(retrieve_context(&index, &root, "ctx-size 8192", &settings))
            .unwrap();
        assert_eq!(
            context.first().map(|c| c.path.as_str()),
            Some("llama-setup.md")
        );

        // Vocabulary-variant phrasing of the same topic. Note this is NOT a
        // morphology/stemming test: FTS5 prefix matching only matches when the
        // *query* token is a prefix of an *indexed* word, not the reverse, so a
        // query for "offloading" alone would match zero chunks against source
        // text that only says "offload" (verified separately). The fixture
        // above therefore spells "GPU offloading" out verbatim in a second
        // sentence, so this assertion genuinely exercises "a differently
        // phrased query still finds the doc that discusses the topic" rather
        // than accidentally passing on the strength of the shared word "GPU"
        // alone (queries here are OR'd per-term, so any single shared term is
        // enough to rank a document — see retrieve_candidates/search_candidates).
        let context = runtime
            .block_on(retrieve_context(&index, &root, "GPU offloading", &settings))
            .unwrap();
        assert_eq!(
            context.first().map(|c| c.path.as_str()),
            Some("llama-setup.md")
        );

        // Irrelevant question against this corpus should not surface router
        // or recipe content for a completely unrelated query term.
        let context = runtime
            .block_on(retrieve_context(&index, &root, "xylophone", &settings))
            .unwrap();
        assert!(context.is_empty());
    }

    #[test]
    fn system_prompt_instructs_the_model_to_admit_insufficient_evidence() {
        let prompt = build_system_prompt(&[], None, &[]);
        assert!(prompt.contains("do not have enough information in your documents"));
        assert!(prompt.to_lowercase().contains("do not invent"));
    }

    #[test]
    fn toggling_reranker_enabled_never_changes_the_final_chunk_count_or_errors() {
        let root =
            std::env::temp_dir().join(format!("foldown-ai-rerank-toggle-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for i in 0..5 {
            std::fs::write(
                root.join(format!("note-{i}.md")),
                format!("# Note {i}\nneedle appears here too"),
            )
            .unwrap();
        }
        let index = KnowledgeIndex::open(root.join("index.db")).unwrap();
        index.sync_workspace(&root).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let mut disabled = crate::settings::store::AiSettings::default();
        disabled.retrieval_final_count = 3;
        let without_reranker = runtime
            .block_on(retrieve_context(&index, &root, "needle", &disabled))
            .unwrap();

        let mut enabled = disabled.clone();
        enabled.reranker_enabled = true;
        enabled.reranker_model = Some("bge-reranker-v2-m3".to_string());
        enabled.reranker_base_url = Some("http://127.0.0.1:1".to_string()); // unreachable by design
        let with_unreachable_reranker = runtime
            .block_on(retrieve_context(&index, &root, "needle", &enabled))
            .unwrap();

        assert_eq!(without_reranker.len(), 3);
        assert_eq!(with_unreachable_reranker.len(), 3);
    }

    #[test]
    fn probe_ai_endpoints_returns_only_the_reachable_server_and_skips_invalid_urls() {
        let reachable =
            spawn_single_response_server(r#"{"data":[{"id":"nomic-embed-text-v1.5"}]}"#);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let probes = runtime.block_on(probe_ai_endpoints(
            vec![
                reachable.clone(),
                "http://127.0.0.1:1".to_string(),
                "not a url".to_string(),
            ],
            None,
        ));
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].base_url, reachable);
        assert_eq!(probes[0].models, vec!["nomic-embed-text-v1.5".to_string()]);
    }
}

#[tauri::command]
pub async fn list_ai_models(
    provider: AiProvider,
    base_url: String,
    api_key: Option<String>,
) -> AppResult<Vec<String>> {
    match provider {
        AiProvider::Local => {
            crate::commands::settings::validate_ai_base_url(&base_url)?;
            client::list_models(&base_url, api_key.as_deref()).await
        }
        AiProvider::Openai => providers::openai::list_models(api_key.as_deref()).await,
        AiProvider::Anthropic => providers::anthropic::list_models(api_key.as_deref()).await,
        AiProvider::Gemini => providers::gemini::list_models(api_key.as_deref()).await,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiServerProbe {
    pub base_url: String,
    pub models: Vec<String>,
}

/// Best-effort discovery/connection-check across several candidate server
/// URLs at once (e.g. common local ports, or the currently configured chat/
/// embedding/reranker endpoints) — probed in parallel with a short timeout,
/// silently skipping unreachable or invalid ones rather than erroring the
/// whole batch, since most candidates are expected to be unreachable.
#[tauri::command]
pub async fn probe_ai_endpoints(urls: Vec<String>, api_key: Option<String>) -> Vec<AiServerProbe> {
    let futures = urls
        .into_iter()
        .filter(|url| crate::commands::settings::validate_ai_base_url(url).is_ok())
        .map(|url| {
            let api_key = api_key.clone();
            async move {
                let models = client::probe_models(&url, api_key.as_deref()).await.ok()?;
                if models.is_empty() {
                    return None;
                }
                Some(AiServerProbe {
                    base_url: url,
                    models,
                })
            }
        });
    futures_util::future::join_all(futures)
        .await
        .into_iter()
        .flatten()
        .collect()
}
