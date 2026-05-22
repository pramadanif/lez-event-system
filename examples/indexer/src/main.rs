//! Reference indexer — demonstrates how to poll a LEZ sequencer RPC and
//! extract events from both successful and failed transactions.
//!
//! # Usage
//!
//! ```bash
//! LEZ_RPC=http://localhost:8545 cargo run --package indexer-example
//! ```
//!
//! The indexer:
//! 1. Polls `/block/latest` every `POLL_INTERVAL_SECS` seconds.
//! 2. For each new block, fetches all transaction hashes.
//! 3. Fetches the `TxReceipt` for each transaction.
//! 4. Extracts events (present even for failed transactions).
//! 5. Stores them in `HashMap<TxHash, Vec<EventRecord>>`.
//! 6. Prints a summary line for each event.

use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use lez_event_decoder::{decode_event, to_display, EventSchema};
use lez_events::EventRecord;
use serde::Deserialize;
use std::collections::HashMap;

const POLL_INTERVAL_SECS: u64 = 5;

// ---------------------------------------------------------------------------
// RPC types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct LatestBlock {
    block_number: u64,
}

#[derive(Debug, Deserialize)]
struct TxReceipt {
    success: bool,
    #[serde(default)]
    events: Vec<EventRaw>,
}

#[derive(Debug, Deserialize)]
struct EventRaw {
    /// Hex-encoded Borsh bytes of an `EventRecord`.
    borsh_hex: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hex_decode(s: &str) -> Result<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).context("hex decode")
}

async fn fetch_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T> {
    let resp = reqwest::get(url)
        .await
        .with_context(|| format!("GET {url}"))?;
    let val = resp.json::<T>().await.context("parse JSON")?;
    Ok(val)
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let rpc = std::env::var("LEZ_RPC").unwrap_or_else(|_| "http://localhost:8545".to_string());
    println!("LEZ Reference Indexer — connecting to {rpc}");

    let schemas: Vec<EventSchema> = vec![]; // extend with known schemas for richer output
    let mut events_by_tx: HashMap<String, Vec<EventRecord>> = HashMap::new();
    let mut last_block: u64 = 0;

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        // Fetch latest block number
        let block_url = format!("{}/block/latest", rpc.trim_end_matches('/'));
        let latest = match fetch_json::<LatestBlock>(&block_url).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[warn] could not fetch latest block: {e}");
                continue;
            }
        };

        if latest.block_number <= last_block {
            continue;
        }

        // Process each new block
        for block_num in (last_block + 1)..=latest.block_number {
            let txs_url = format!("{}/block/{}/txs", rpc.trim_end_matches('/'), block_num);
            let tx_hashes: Vec<String> = match fetch_json(&txs_url).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[warn] block {block_num} tx list: {e}");
                    continue;
                }
            };

            for tx_hash in tx_hashes {
                let receipt_url =
                    format!("{}/tx/{}", rpc.trim_end_matches('/'), &tx_hash);
                let receipt: TxReceipt = match fetch_json(&receipt_url).await {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("[warn] receipt {tx_hash}: {e}");
                        continue;
                    }
                };

                let mut records: Vec<EventRecord> = Vec::new();
                for raw in &receipt.events {
                    match hex_decode(&raw.borsh_hex)
                        .and_then(|b| EventRecord::deserialize(&mut &b[..]).context("borsh"))
                    {
                        Ok(r) => records.push(r),
                        Err(e) => eprintln!("[warn] decode event in {tx_hash}: {e}"),
                    }
                }

                if !records.is_empty() {
                    for record in &records {
                        let mut decoded = decode_event(record, &schemas);
                        decoded.from_failed_tx = !receipt.success;
                        print!(
                            "[block={}] [tx={}] {}",
                            block_num,
                            &tx_hash[..8.min(tx_hash.len())],
                            to_display(&decoded)
                        );
                    }
                    events_by_tx.insert(tx_hash, records);
                }
            }
        }

        last_block = latest.block_number;
        println!(
            "[indexer] processed up to block {last_block}, total txs with events: {}",
            events_by_tx.len()
        );
    }
}
