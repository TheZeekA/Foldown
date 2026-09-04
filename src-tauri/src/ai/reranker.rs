use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::settings::store::AiSettings;

#[derive(Serialize)]
struct RerankRequest<'a> {
    model: &'a str,
    query: &'a str,
    documents: &'a [String],
}

/// Parses llama.cpp's TEI/Jina-compatible `/rerank` response shape:
/// `{"results": [{"index": N, "relevance_score": f64}, ...]}`. Verified
/// against the installed llama.cpp build per this plan's Task 11 — if a
/// different build returns a different shape, this is the only function
/// that needs to change.
fn parse_rerank_response(json: &str, document_count: usize) -> AppResult<Vec<f32>> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| AppError::Message("The reranker returned invalid JSON".to_string()))?;
    let results = value
        .get("results")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            AppError::Message("The reranker response contained no results".to_string())
        })?;
    let mut scores = vec![0.0f32; document_count];
    for item in results {
        let index = item.get("index").and_then(|v| v.as_u64()).ok_or_else(|| {
            AppError::Message("A reranker result was missing its index".to_string())
        })? as usize;
        let score = item
            .get("relevance_score")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                AppError::Message("A reranker result was missing relevance_score".to_string())
            })? as f32;
        if let Some(slot) = scores.get_mut(index) {
            *slot = score;
        }
    }
    Ok(scores)
}

pub async fn rerank(
    settings: &AiSettings,
    model: &str,
    query: &str,
    documents: &[String],
) -> AppResult<Vec<f32>> {
    if documents.is_empty() {
        return Ok(Vec::new());
    }
    let base_url = settings
        .reranker_base_url
        .as_deref()
        .unwrap_or(&settings.local.base_url);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Message(format!("Could not configure the reranker client: {e}")))?;
    let mut request = client
        .post(crate::ai::client::endpoint(base_url, "rerank")?)
        .json(&RerankRequest {
            model,
            query,
            documents,
        });
    if let Some(key) = settings.local.api_key.as_deref() {
        request = request.bearer_auth(key);
    }
    let response = request
        .send()
        .await
        .map_err(|e| AppError::Message(format!("Could not connect to the reranker: {e}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::Message(format!("Could not read the reranker response: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Message(format!(
            "Reranker returned HTTP {}",
            status.as_u16()
        )));
    }
    parse_rerank_response(&body, documents.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reranker_scores_back_into_input_order() {
        let json = r#"{"results":[{"index":1,"relevance_score":0.87},{"index":0,"relevance_score":0.12}]}"#;
        assert_eq!(parse_rerank_response(json, 2).unwrap(), vec![0.12, 0.87]);
    }

    #[test]
    fn missing_results_default_to_zero_relevance_rather_than_erroring() {
        let json = r#"{"results":[{"index":0,"relevance_score":0.5}]}"#;
        assert_eq!(parse_rerank_response(json, 3).unwrap(), vec![0.5, 0.0, 0.0]);
    }

    #[test]
    fn rejects_malformed_reranker_json() {
        assert!(parse_rerank_response("not json", 1).is_err());
    }
}
