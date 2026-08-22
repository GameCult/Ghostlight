use anyhow::{Context, Result, bail};
use ghostlight_dungeon::{persistence::CampaignStore, session_zero::SessionZeroState};
use serde_json::json;

fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let store_path = arguments
        .next()
        .context("usage: ghostlight-session-zero-inspect <session-zero.cc> [receipt-limit]")?;
    let limit = arguments
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .context("receipt limit must be a positive integer")?
        .unwrap_or(8);
    if limit == 0 {
        bail!("receipt limit must be greater than zero");
    }

    let store = CampaignStore::open(store_path)?;
    let key = store
        .keys("session_zero.v1")?
        .into_iter()
        .next()
        .context("store contains no session_zero.v1 state")?;
    let (_, session) = store
        .load::<SessionZeroState>("session_zero.v1", &key)?
        .context("session_zero.v1 row disappeared during inspection")?;
    let first = session.preview_model_receipts.len().saturating_sub(limit);
    let receipts = session.preview_model_receipts[first..]
        .iter()
        .map(|receipt| {
            json!({
                "stage": receipt.stage,
                "provider": receipt.provider,
                "model": receipt.model,
                "requestHash": receipt.request_hash,
                "outputHash": receipt.output_hash,
                "latencyMs": receipt.latency_ms,
                "validationResult": receipt.validation_result,
                "inputChars": receipt.input_chars,
                "outputChars": receipt.output_chars,
                "attempts": receipt.provider_attempts.iter().map(|attempt| json!({
                    "finishReason": attempt.finish_reason,
                    "latencyMs": attempt.latency_ms,
                    "tokenUsage": attempt.token_usage,
                    "localValidationResult": attempt.local_validation_result,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "ghostlight.session_zero_receipt_inspection.v1",
            "sessionId": session.id,
            "revision": session.revision,
            "status": session.status,
            "receiptCount": session.preview_model_receipts.len(),
            "receipts": receipts,
        }))?
    );
    Ok(())
}
