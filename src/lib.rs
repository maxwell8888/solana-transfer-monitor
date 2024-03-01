use anyhow::{bail, Result};
use instructions::handle_parsed_instruction;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransaction, EncodedTransactionWithStatusMeta,
    TransactionDetails, UiConfirmedBlock, UiInstruction, UiMessage, UiParsedInstruction,
    UiTransactionEncoding,
};
use std::{
    collections::HashMap,
    io::{self, Write},
    thread::sleep,
    time::{Duration, Instant},
};
use tracing::{debug, info, trace};
use tracing_subscriber::EnvFilter;

pub mod instructions;
pub mod utils;

const USDC_MINT_ADDRESS: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

// https://solana.com/docs/core/clusters
const RATE_LIMIT_PERIOD: u64 = 10;
// const MAX_REQUESTS_PER_PERIOD: usize = 100;
const MAX_REQUESTS_PER_PERIOD: usize = 40;

pub fn run() -> Result<()> {
    if let Ok(level) = std::env::var("RUST_LOG") {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(format!("solana_transfer_monitor={level}")))
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

pub fn make_block_config() -> RpcBlockConfig {
    let mut rpc_block_config = RpcBlockConfig::default();
    rpc_block_config.encoding = Some(UiTransactionEncoding::JsonParsed);
    rpc_block_config.transaction_details = Some(TransactionDetails::Full);
    rpc_block_config.max_supported_transaction_version = Some(0);
    rpc_block_config
}

fn check_request_instants(request_instants: &mut Vec<Instant>) {
    trace!("checking request instants");
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
                OptionSerializer::Some(token_balances) => {
                    for token_balance in token_balances {
                        if token_balance.mint == USDC_MINT_ADDRESS {
                            let pub_key = parsed_accounts[token_balance.account_index as usize]
                                .pubkey
                                .clone();
                            let owner = match token_balance.owner {
                                OptionSerializer::Some(owner) => owner,
                                _ => bail!("expected OptionSerializer::Some"),
                            };
                            accounts_map.insert(pub_key, (owner, token_balance.mint));
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
