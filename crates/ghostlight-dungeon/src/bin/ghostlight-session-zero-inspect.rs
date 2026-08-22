use anyhow::{Context, Result, bail};
use ghostlight_dungeon::{persistence::CampaignStore, session_zero::SessionZeroState};
use serde_json::json;
use std::collections::BTreeMap;

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
    let legacy_receipt_layout =
        session.model_receipts.is_empty() && !session.preview_model_receipts.is_empty();
    let audit_receipts = if legacy_receipt_layout {
        &session.preview_model_receipts
    } else {
        &session.model_receipts
    };
    let first = audit_receipts.len().saturating_sub(limit);
    let mut receipt_key_counts = BTreeMap::new();
    for receipt in audit_receipts {
        *receipt_key_counts
            .entry(receipt.storage_key().to_owned())
            .or_insert(0usize) += 1;
    }
    let duplicate_receipt_keys = receipt_key_counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .collect::<BTreeMap<_, _>>();
    let receipts = audit_receipts[first..]
        .iter()
        .map(|receipt| {
            json!({
                "storageKey": receipt.storage_key(),
                "stage": receipt.stage,
                "provider": receipt.provider,
                "model": receipt.model,
                "snapshotBinding": receipt.snapshot_binding,
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
            "receiptCount": audit_receipts.len(),
            "activePreviewReceiptCount": session.preview_model_receipts.len(),
            "legacyReceiptLayout": legacy_receipt_layout,
            "duplicateReceiptKeys": duplicate_receipt_keys,
            "receipts": receipts,
        }))?
    );
    Ok(())
}
