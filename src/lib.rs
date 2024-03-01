use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransaction, EncodedTransactionWithStatusMeta,
    TransactionDetails, UiConfirmedBlock, UiInstruction, UiMessage, UiParsedInstruction,
    UiTransactionEncoding,
};
use std::{
    collections::HashMap,
    fmt,
    io::{self, Write},
    str::from_utf8,
    thread::sleep,
    time::{Duration, Instant},
};
pub mod utils;
use tracing::{debug, info, trace};
use tracing_subscriber::EnvFilter;

const USDC_MINT_ADDRESS: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

// https://solana.com/docs/core/clusters
const RATE_LIMIT_PERIOD: u64 = 10;
// const MAX_REQUESTS_PER_PERIOD: usize = 100;
const MAX_REQUESTS_PER_PERIOD: usize = 40;

pub fn make_block_config() -> RpcBlockConfig {
    let mut rpc_block_config = RpcBlockConfig::default();
    rpc_block_config.encoding = Some(UiTransactionEncoding::JsonParsed);
    rpc_block_config.transaction_details = Some(TransactionDetails::Full);
    rpc_block_config.max_supported_transaction_version = Some(0);
    rpc_block_config
}

fn write_transaction_transfers<W: Write>(
    transaction: EncodedTransactionWithStatusMeta,
    writer: &mut W,
) -> Result<()> {
    let signature = match &transaction.transaction {
        EncodedTransaction::Json(ui_transaction) => ui_transaction.signatures.clone(),
        _ => bail!("expected EncodedTransaction::Json"),
    };

    let mut accounts_map = HashMap::new();

    let parsed_accounts = match transaction.transaction {
        EncodedTransaction::Json(ui_transaction) => match ui_transaction.message {
            UiMessage::Parsed(ui_parsed_message) => ui_parsed_message.account_keys,
            _ => bail!("expected UiMessage::Parsed"),
        },
        _ => bail!("expected EncodedTransaction::Json"),
    };

    if let Some(meta) = transaction.meta {
        if meta.err.is_none() {
            match meta.pre_token_balances {
                OptionSerializer::Some(ui_transaction_token_balances) => {
                    for pre_ui_transaction_token_balance in ui_transaction_token_balances {
                        if pre_ui_transaction_token_balance.mint == USDC_MINT_ADDRESS {
                            let pub_key = parsed_accounts
                                [pre_ui_transaction_token_balance.account_index as usize]
                                .pubkey
                                .clone();
                            let owner = match pre_ui_transaction_token_balance.owner {
                                OptionSerializer::Some(owner) => owner,
                                _ => bail!("expected OptionSerializer::Some"),
                            };
                            accounts_map
                                .insert(pub_key, (owner, pre_ui_transaction_token_balance.mint));
                        }
                    }
                }
                _ => bail!("expected OptionSerializer::Some"),
            }

            let mut first_transfer = true;

            match meta.inner_instructions {
                OptionSerializer::Some(intructions_vec) => {
                    for instructions in intructions_vec {
                        for instruction in instructions.instructions {
                            match instruction {
                                UiInstruction::Compiled(_) => {
                                    bail!("expected UiInstruction::Parsed")
                                }
                                UiInstruction::Parsed(ui_instruction_parsed) => {
                                    match ui_instruction_parsed {
                                        UiParsedInstruction::Parsed(parsed_instruction) => {
                                            if parsed_instruction.program == "spl-token" {
                                                let transfer = handle_parsed_instruction(
                                                    parsed_instruction.parsed,
                                                    &mut accounts_map,
                                                )?;
                                                if let Some(transfer) = transfer {
                                                    if first_transfer {
                                                        debug!("tx signature: {signature:?}");
                                                        first_transfer = false;
                                                    }
                                                    writeln!(writer, "{transfer}")?;
                                                }
                                            }
                                        }
                                        UiParsedInstruction::PartiallyDecoded(_) => {}
                                    }
                                }
                            }
                        }
                    }
                }
                _ => bail!("expected OptionSerializer::Some"),
            }
        }
    }
    Ok(())
}

pub fn write_block_transfers<W: Write>(
    block: UiConfirmedBlock,
    slot: u64,
    writer: &mut W,
) -> Result<()> {
    writeln!(writer, "Latest block: {slot}")?;

    if let Some(transactions) = block.transactions {
        for transaction in transactions {
            write_transaction_transfers(transaction, writer)?;
        }
    } else {
        info!("no transactions found for block in slot {slot}");
    }

    Ok(())
}

fn check_request_instants(request_instants: &mut Vec<Instant>) {
    trace!("checking request intants");
    // Remove requests older than the rate limit period (10secs)
    for i in 0..request_instants.len() {
        if request_instants[i].elapsed() < Duration::from_secs(RATE_LIMIT_PERIOD) {
            request_instants.drain(..i);
            break;
        }
    }
    if request_instants.len() > MAX_REQUESTS_PER_PERIOD {
        // Exceeded rate limit so wait a while before the next iteration to avoid rate limiting
        trace!("exceeded rate limit, waiting 500ms");
        sleep(Duration::from_millis(500));
        check_request_instants(request_instants);
    }
}

pub fn run() -> Result<()> {
    if let Ok(level) = std::env::var("RUST_LOG") {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(&format!("solana_transfer_monitor={level}")))
            .init();
    }

    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    // let rpc_url = "https://api.devnet.solana.com".to_string();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::finalized());

    // Possible solution to rate limiting but doesn't appear to work for get_block
    // let client = RpcClient::new_with_timeout_and_commitment(
    //     rpc_url,
    //     Duration::from_secs(2),
    //     CommitmentConfig::finalized(),
    // );

    let rpc_block_config = make_block_config();
    let mut starting_slot = client.get_slot()?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let mut request_instants: Vec<Instant> = Vec::new();

    loop {
        let iteration_start = Instant::now();

        check_request_instants(&mut request_instants);

        let slots = client.get_blocks(starting_slot, None)?;
        debug!("client.get_blocks slots.len(): {}", slots.len());
        request_instants.push(Instant::now());

        if let Some(last_slot) = slots.last() {
            trace!("increment starting slot");
            starting_slot = last_slot + 1;
        } else {
            trace!("no slots returned, waiting 500ms");
            sleep(Duration::from_millis(500));
            continue;
        }

        for slot in slots.clone() {
            check_request_instants(&mut request_instants);

            let get_block_start = Instant::now();
            trace!("request block {slot}");
            let block = client.get_block_with_config(slot, rpc_block_config)?;
            request_instants.push(Instant::now());
            trace!(
                "get_block_with_config took: {:?}",
                get_block_start.elapsed()
            );
            write_block_transfers(block, slot, &mut handle)?;
        }

        trace!("loop iteration elapsed in {:?}", iteration_start.elapsed());
    }
}

// >8 -> x,xxx,xxx.xx (remove decimals if 00)
// 7 -> x.xxx x
// 6 -> 0.xxx xxx
// 5 -> 0.0xx xxx
// 4 -> 0.00x xxx
// etc
fn format_amount(raw_amount: &str) -> Result<String> {
    Ok(match raw_amount.len() {
        7 => format!("{}.{}", &raw_amount[..1], &raw_amount[1..5]),
        6 => format!("0.{raw_amount}"),
        5 => format!("0.0{raw_amount}"),
        4 => format!("0.00{raw_amount}"),
        3 => format!("0.000{raw_amount}"),
        2 => format!("0.000{raw_amount}"),
        1 => raw_amount.to_string(),
        _ => {
            let pre_decy = raw_amount[..raw_amount.len() - 6]
                .as_bytes()
                .rchunks(3)
                .rev()
                .map(from_utf8)
                .collect::<Result<Vec<&str>, _>>()?
                .join(",");
            let post_decy = &raw_amount[raw_amount.len() - 6..raw_amount.len() - 4];
            if post_decy == "00" {
                pre_decy
            } else {
                format!("{}.{}", pre_decy, post_decy)
            }
        }
    })
}
pub struct Transfer {
    pub source_owner: String,
    pub destination_owner: String,
    pub formatted_amount: String,
}
impl fmt::Display for Transfer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Transfer {
            source_owner,
            destination_owner,
            formatted_amount,
        } = self;
        write!(
            f,
            "TX detected: {source_owner} sent {formatted_amount} USDC to {destination_owner}"
        )
    }
}

fn handle_parsed_instruction(
    mut parsed_instruction: Value,
    accounts_map: &mut HashMap<String, (String, String)>,
) -> Result<Option<Transfer>> {
    let type_ = parsed_instruction["type"].take();
    let type_ = type_
        .as_str()
        .context("type not found in instruction JSON")?;

    if type_ == "transfer" || type_ == "transferChecked" {
        let mut info = parsed_instruction["info"].take();

        let source = info["source"].take();
        let err_message = "source not found in instruction JSON";
        let source = source.as_str().context(err_message)?;

        let destination = info["destination"].take();
        let err_message = "destination not found in instruction JSON";
        let destination = destination.as_str().context(err_message)?;

        // we only want to handle USDC transfers, but we don't know the mint key until we lookup the source and destination in accounts_mapping
        // Instruction might not be for the correct mint, so might not exist in accounts_map
        let Some((source_owner, source_mint)) = accounts_map.get(source) else {
            return Ok(None);
        };
        let Some((destination_owner, destination_mint)) = accounts_map.get(destination) else {
            return Ok(None);
        };

        if source_mint != destination_mint {
            return Err(anyhow!("source and destination mint do not match"));
        }

        if source_mint == USDC_MINT_ADDRESS {
            let (raw_amount, message) = if type_ == "transfer" {
                let raw_amount = info["amount"].take();
                let message = "amount not found in instruction JSON";
                (raw_amount, message)
            } else {
                let mut token_amount = info["tokenAmount"].take();
                let raw_amount = token_amount["amount"].take();
                let message = "amount not found in tokenAmount JSON";
                (raw_amount, message)
            };
            let raw_amount = raw_amount.as_str().context(message)?;

            let formatted_amount = format_amount(raw_amount)?;
            return Ok(Some(Transfer {
                source_owner: source_owner.clone(),
                destination_owner: destination_owner.clone(),
                formatted_amount,
            }));
        }
    }
    Ok(None)
}
