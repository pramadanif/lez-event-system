/// lez-event-cli — decode and watch LEZ program events.
///
/// # Usage
///
/// ```text
/// lez-event-cli decode --tx <TX_HASH> --rpc <RPC_URL> [--format json|display]
/// lez-event-cli decode-raw --hex <HEX_BYTES>
/// lez-event-cli decode-raw --file <PATH>
/// lez-event-cli watch [--program <PROGRAM_ID>] --rpc <RPC_URL>
/// ```
use anyhow::{bail, Context, Result};
use borsh::BorshDeserialize;
use clap::{Parser, Subcommand};
use lez_event_decoder::{decode_event, to_display, to_json, EventSchema};
use lez_events::EventRecord;
use lez_events_runtime::parse_journal;
use serde::Deserialize;

#[derive(Parser)]
#[command(name = "lez-event-cli")]
#[command(version = "0.1.0")]
#[command(about = "Decode and watch LEZ program events", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Decode events from a transaction hash (fetches receipt from RPC).
    Decode {
        /// Transaction hash (hex, with or without 0x prefix).
        #[arg(long)]
        tx: String,
        /// RPC endpoint URL, e.g. http://localhost:8545.
        #[arg(long)]
        rpc: String,
        /// Output format: `display` (default) or `json`.
        #[arg(long, default_value = "display")]
        format: String,
    },

    /// Decode events from raw framed journal bytes.
    DecodeRaw {
        /// Hex-encoded journal bytes containing the LEZE event frame.
        #[arg(long, conflicts_with = "file")]
        hex: Option<String>,
        /// Path to a binary file containing the journal.
        #[arg(long)]
        file: Option<String>,
    },

    /// Watch for new events in real-time by polling the RPC.
    Watch {
        /// Only show events from this program ID (hex).
        #[arg(long)]
        program: Option<String>,
        /// RPC endpoint URL.
        #[arg(long)]
        rpc: String,
        /// Poll interval in seconds (default: 5).
        #[arg(long, default_value = "5")]
        interval: u64,
    },
}

// ---------------------------------------------------------------------------
// Minimal RPC response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TxReceiptResponse {
    success: bool,
    #[serde(default)]
    events: Vec<EventRecordRaw>,
}

/// Raw event as returned by the RPC before local Borsh decode.
#[derive(Debug, Deserialize)]
struct EventRecordRaw {
    /// Hex-encoded Borsh bytes of the whole EventRecord.
    borsh_hex: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn no_schemas() -> Vec<EventSchema> {
    vec![]
}

/// Strip optional `0x` prefix and hex-decode.
fn hex_decode(s: &str) -> Result<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).context("invalid hex string")
}

fn print_events(records: &[EventRecord], schemas: &[EventSchema], from_failed: bool, format: &str) {
    if records.is_empty() {
        println!("(no events)");
        return;
    }
    for record in records {
        let mut decoded = decode_event(record, schemas);
        decoded.from_failed_tx = from_failed;
        match format {
            "json" => println!("{}", to_json(&decoded)),
            _ => print!("{}", to_display(&decoded)),
        }
    }
}

fn records_from_journal(bytes: &[u8]) -> Result<Vec<EventRecord>> {
    let (events, _) = parse_journal(bytes).context("failed to parse framed journal")?;
    Ok(events)
}

// ---------------------------------------------------------------------------
// Subcommand implementations
// ---------------------------------------------------------------------------

async fn cmd_decode(tx: &str, rpc: &str, format: &str) -> Result<()> {
    let url = format!("{}/tx/{}", rpc.trim_end_matches('/'), tx);
    let resp = reqwest::get(&url)
        .await
        .with_context(|| format!("GET {url}"))?;

    if !resp.status().is_success() {
        bail!("RPC returned HTTP {}: {}", resp.status(), url);
    }

    let receipt: TxReceiptResponse = resp.json().await.context("parse TxReceipt JSON")?;
    let schemas = no_schemas();

    // Decode each raw event record
    let mut records: Vec<EventRecord> = Vec::new();
    for raw in &receipt.events {
        let bytes = hex_decode(&raw.borsh_hex)?;
        let r = EventRecord::deserialize(&mut &bytes[..]).context("Borsh decode EventRecord")?;
        records.push(r);
    }

    print_events(&records, &schemas, !receipt.success, format);
    Ok(())
}

async fn cmd_decode_raw(hex_arg: Option<&str>, file_arg: Option<&str>) -> Result<()> {
    let bytes = match (hex_arg, file_arg) {
        (Some(h), _) => hex_decode(h)?,
        (_, Some(f)) => std::fs::read(f).with_context(|| format!("read {f}"))?,
        (None, None) => bail!("provide --hex or --file"),
    };

    let records = records_from_journal(&bytes)?;
    let schemas = no_schemas();
    print_events(&records, &schemas, false, "display");
    Ok(())
}

async fn cmd_watch(program_filter: Option<&str>, rpc: &str, interval_secs: u64) -> Result<()> {
    use tokio::time::{sleep, Duration};

    let filter_bytes: Option<[u8; 32]> = program_filter
        .map(|s| {
            let b = hex_decode(s)?;
            if b.len() != 32 {
                bail!("program id must be 32 bytes");
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            Ok(arr)
        })
        .transpose()?;

    println!(
        "Watching {} for events (poll every {}s) …",
        rpc, interval_secs
    );

    let schemas = no_schemas();
    let mut last_seen_block: u64 = 0;

    loop {
        let block_url = format!("{}/block/latest", rpc.trim_end_matches('/'));
        if let Ok(resp) = reqwest::get(&block_url).await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let block_num = json["block_number"].as_u64().unwrap_or(last_seen_block);
                if block_num > last_seen_block {
                    for b in (last_seen_block + 1)..=block_num {
                        let txs_url = format!("{}/block/{}/txs", rpc.trim_end_matches('/'), b);
                        if let Ok(r) = reqwest::get(&txs_url).await {
                            if let Ok(tx_list) = r.json::<Vec<String>>().await {
                                for tx_hash in tx_list {
                                    if let Ok(receipt) = fetch_receipt(rpc, &tx_hash).await {
                                        let mut records: Vec<EventRecord> = Vec::new();
                                        for raw in &receipt.events {
                                            if let Ok(bytes) = hex_decode(&raw.borsh_hex) {
                                                if let Ok(r) =
                                                    EventRecord::deserialize(&mut &bytes[..])
                                                {
                                                    records.push(r);
                                                }
                                            }
                                        }
                                        // Apply program filter
                                        let filtered: Vec<&EventRecord> = records
                                            .iter()
                                            .filter(|r| {
                                                filter_bytes
                                                    .map(|f| r.program_id == f)
                                                    .unwrap_or(true)
                                            })
                                            .collect();
                                        if !filtered.is_empty() {
                                            println!("[block={b}] [tx={tx_hash}]");
                                            for r in filtered {
                                                let mut d = decode_event(r, &schemas);
                                                d.from_failed_tx = !receipt.success;
                                                print!("{}", to_display(&d));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    last_seen_block = block_num;
                }
            }
        }
        sleep(Duration::from_secs(interval_secs)).await;
    }
}

async fn fetch_receipt(rpc: &str, tx: &str) -> Result<TxReceiptResponse> {
    let url = format!("{}/tx/{}", rpc.trim_end_matches('/'), tx);
    let r = reqwest::get(&url)
        .await?
        .json::<TxReceiptResponse>()
        .await?;
    Ok(r)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Decode { tx, rpc, format } => cmd_decode(tx, rpc, format).await,
        Commands::DecodeRaw { hex, file } => cmd_decode_raw(hex.as_deref(), file.as_deref()).await,
        Commands::Watch {
            program,
            rpc,
            interval,
        } => cmd_watch(program.as_deref(), rpc, *interval).await,
    };

    if let Err(e) = result {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
