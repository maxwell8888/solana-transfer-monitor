use anyhow::{Ok, Result};
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
    io::{self, Write},
    str::from_utf8, // Correctly import from_utf8
};

// TODO NOTE: "The program should begin tracking from the latest block" note latest *block*, not slot
// TODO graceful shutdown
// TODO add logic to prevent exceeding rate limit, ie count number of requests every 10 seconds
// TODO https://solscan.io/tx/3ybfF... seems to just use one sig from the transaction, but the API provides a Vec... when can there be more than 1 sig?
// TODO combine transfers between the same accounts in the same direction in the same transaction?
// TODO techinically we don't need to keep asking for slot numbers, we can just calculate them ourselves and save 1 request for the rate limiting

// USDC
const TOKEN_MINT_ADDRESS: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

pub fn make_block_config() -> RpcBlockConfig {
    let mut rpc_block_config = RpcBlockConfig::default();
    rpc_block_config.encoding = Some(UiTransactionEncoding::JsonParsed);
    rpc_block_config.transaction_details = Some(TransactionDetails::Full);
    rpc_block_config.max_supported_transaction_version = Some(0);
    rpc_block_config
}

pub fn get_all_successful_usdc_transactions(block: UiConfirmedBlock) -> Vec<String> {
    let mut transaction_signatures = Vec::new();
    if let Some(transactions) = block.transactions {
        for transaction in transactions {
            // print_transaction_transfers(transaction);
            let debug_string = format!("{transaction:?}");

            if transaction.meta.unwrap().err.is_none() && debug_string.contains(TOKEN_MINT_ADDRESS)
            {
                let signature = match &transaction.transaction {
                    EncodedTransaction::LegacyBinary(_) => todo!(),
                    EncodedTransaction::Binary(_, _) => todo!(),
                    EncodedTransaction::Json(ui_transaction) => ui_transaction.signatures.clone(),
                    EncodedTransaction::Accounts(_) => todo!(),
                };
                if signature.len() > 1 {
                    // TODO
                    transaction_signatures.push(signature.first().unwrap().clone());
                } else {
                    transaction_signatures.push(signature.first().unwrap().clone());
                }
            }
        }
    } else {
        eprintln!("Error: no transactions found for block");
    }
    transaction_signatures
}

fn write_transaction_transfers<W: Write>(
    transaction: EncodedTransactionWithStatusMeta,
    writer: &mut W,
) -> Result<()> {
    let signature = match &transaction.transaction {
        EncodedTransaction::LegacyBinary(_) => todo!(),
        EncodedTransaction::Binary(_, _) => todo!(),
        EncodedTransaction::Json(ui_transaction) => ui_transaction.signatures.clone(),
        EncodedTransaction::Accounts(_) => todo!(),
    };

    let mut accounts_map = HashMap::new();

    let parsed_accounts = match transaction.transaction {
        EncodedTransaction::Json(ui_transaction) => match ui_transaction.message {
            UiMessage::Parsed(ui_parsed_message) => ui_parsed_message.account_keys,
            UiMessage::Raw(_) => todo!(),
        },
        _ => todo!(),
    };

    if let Some(meta) = transaction.meta {
        if meta.err.is_none() {
            match meta.pre_token_balances {
                OptionSerializer::Some(ui_transaction_token_balances) => {
                    for pre_ui_transaction_token_balance in ui_transaction_token_balances {
                        if pre_ui_transaction_token_balance.mint == TOKEN_MINT_ADDRESS {
                            let pub_key = parsed_accounts
                                [pre_ui_transaction_token_balance.account_index as usize]
                                .pubkey
                                .clone();
                            let owner = match pre_ui_transaction_token_balance.owner {
                                OptionSerializer::Some(owner) => owner,
                                _ => todo!(),
                            };
                            accounts_map
                                .insert(pub_key, (owner, pre_ui_transaction_token_balance.mint));
                        }
                    }
                }
                OptionSerializer::None => todo!(),
                OptionSerializer::Skip => todo!(),
            }

            let mut print_sig = true;

            match meta.inner_instructions {
                OptionSerializer::Some(intructions_vec) => {
                    for instructions in intructions_vec {
                        for intruction in instructions.instructions {
                            match intruction {
                                UiInstruction::Compiled(_) => {
                                    todo!()
                                }
                                UiInstruction::Parsed(ui_instruction_parsed) => {
                                    match ui_instruction_parsed {
                                        UiParsedInstruction::Parsed(parsed_instruction) => {
                                            if parsed_instruction.program == "spl-token" {
                                                let transfer = handle_parsed_instruction(
                                                    &parsed_instruction.parsed,
                                                    &accounts_map,
                                                );
                                                if let Some(transfer) = transfer {
                                                    if print_sig {
                                                        // println!("");
                                                        // println!("signature: {signature:?}");
                                                        print_sig = false;
                                                    }
                                                    transfer.write(writer)?;
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
                OptionSerializer::None => todo!(),
                OptionSerializer::Skip => todo!(),
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
        eprintln!("Error: no transactions found for block in slot {slot}");
    }

    Ok(())
}

pub async fn run() -> Result<()> {
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    // let rpc_url = "https://api.devnet.solana.com".to_string();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::finalized());

    let _current_slot = client.get_slot().unwrap();

    // block slot with lots of USDC transfers: 250655260
    let _slot = 250655260;

    // block slot with a few USDC transfers: 250684537
    let slot = 250684537;

    let block = client.get_block_with_config(slot, make_block_config())?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    write_block_transfers(block, slot, &mut handle)?;

    // let transactions = get_all_successful_usdc_transactions(block);
    // for t in transactions {
    //     println!("{t}");
    // }

    Ok(())
}

// >8 -> xx.xx (remove decimals if 00)
// 7 -> x.xxx x
// 6 -> 0.xxx xxx
fn format_amount(raw_amount: &str) -> String {
    let amount = if raw_amount.len() > 7 {
        let pre_decy = raw_amount[..raw_amount.len() - 6]
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(from_utf8)
            .collect::<Result<Vec<&str>, _>>()
            .unwrap()
            .join(",");
        let post_decy = &raw_amount[raw_amount.len() - 6..raw_amount.len() - 4];
        if post_decy == "00" {
            pre_decy
        } else {
            format!("{}.{}", pre_decy, post_decy)
        }
    } else if raw_amount.len() == 7 {
        format!("{}.{}", &raw_amount[..1], &raw_amount[1..5])
    } else if raw_amount.len() == 6 {
        format!("0.{raw_amount}")
    } else {
        dbg!(raw_amount);
        todo!()
    };
    amount
}
pub struct Transfer {
    pub source_owner: String,
    pub destination_owner: String,
    pub formatted_amount: String,
}
impl Transfer {
    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let Transfer {
            source_owner,
            destination_owner,
            formatted_amount,
        } = self;

        writeln!(
            writer,
            "TX detected: {source_owner} sent {formatted_amount} USDC to {destination_owner}"
        )?;

        Ok(())
    }
}

fn handle_parsed_instruction(
    parsed_instruction: &Value,
    accounts_map: &HashMap<String, (String, String)>,
) -> Option<Transfer> {
    match parsed_instruction {
        Value::Object(map) => {
            if let Some(type_) = map.get("type") {
                // we only want to handle USDC transfers, but we don't know the mint key until we lookup the source and destination in accounts_mapping
                if type_ == "transfer" {
                    if let Some(info) = map.get("info") {
                        match info {
                            Value::Object(info_map) => {
                                let (source_owner, source_mint) =
                                    if let Some(source) = info_map.get("source") {
                                        match source {
                                            Value::String(source) => {
                                                if let Some(thing) = accounts_map.get(source) {
                                                    thing.clone()
                                                } else {
                                                    return None;
                                                }
                                            }
                                            _ => todo!(),
                                        }
                                    } else {
                                        todo!()
                                    };
                                let (destination_owner, destination_mint) =
                                    if let Some(destination) = info_map.get("destination") {
                                        match destination {
                                            Value::String(destination) => {
                                                if let Some(thing) = accounts_map.get(destination) {
                                                    thing.clone()
                                                } else {
                                                    return None;
                                                }
                                            }
                                            _ => todo!(),
                                        }
                                    } else {
                                        todo!()
                                    };
                                let raw_amount = if let Some(amount) = info_map.get("amount") {
                                    match amount {
                                        Value::String(amount) => amount.clone(),
                                        _ => todo!(),
                                    }
                                } else {
                                    todo!()
                                };

                                if source_mint != destination_mint {
                                    panic!("source and destination mints do no match");
                                } else if source_mint == TOKEN_MINT_ADDRESS {
                                    Some(Transfer {
                                        source_owner,
                                        destination_owner,
                                        formatted_amount: format_amount(&raw_amount),
                                    })
                                } else {
                                    None
                                }
                            }
                            _ => todo!(),
                        }
                    } else {
                        todo!()
                    }
                } else if type_ == "transferChecked" {
                    // TODO
                    if let Some(info) = map.get("info") {
                        match info {
                            Value::Object(info_map) => {
                                let mint = if let Some(mint) = info_map.get("mint") {
                                    match mint {
                                        Value::String(mint) => mint.clone(),
                                        _ => todo!(),
                                    }
                                } else {
                                    todo!()
                                };

                                if mint == TOKEN_MINT_ADDRESS {
                                    // println!("transferChecked");

                                    // TODO some of this is duplicated from above
                                    if let Some(info) = map.get("info") {
                                        match info {
                                            Value::Object(info_map) => {
                                                let (source_owner, source_mint) =
                                                    if let Some(source) = info_map.get("source") {
                                                        match source {
                                                            Value::String(source) => {
                                                                if let Some(thing) =
                                                                    accounts_map.get(source)
                                                                {
                                                                    thing.clone()
                                                                } else {
                                                                    return None;
                                                                }
                                                            }
                                                            _ => todo!(),
                                                        }
                                                    } else {
                                                        todo!()
                                                    };
                                                let (destination_owner, destination_mint) =
                                                    if let Some(destination) =
                                                        info_map.get("destination")
                                                    {
                                                        match destination {
                                                            Value::String(destination) => {
                                                                if let Some(thing) =
                                                                    accounts_map.get(destination)
                                                                {
                                                                    thing.clone()
                                                                } else {
                                                                    return None;
                                                                }
                                                            }
                                                            _ => todo!(),
                                                        }
                                                    } else {
                                                        todo!()
                                                    };
                                                let raw_amount = if let Some(token_amount) =
                                                    info_map.get("tokenAmount")
                                                {
                                                    token_amount
                                                        .get("amount")
                                                        .unwrap()
                                                        .as_str()
                                                        .unwrap()
                                                        .to_string()
                                                } else {
                                                    todo!()
                                                };

                                                if source_mint != destination_mint {
                                                    panic!(
                                                        "source and destination mints do no match"
                                                    );
                                                } else if source_mint == TOKEN_MINT_ADDRESS {
                                                    Some(Transfer {
                                                        source_owner,
                                                        destination_owner,
                                                        formatted_amount: format_amount(
                                                            &raw_amount,
                                                        ),
                                                    })
                                                } else {
                                                    None
                                                }
                                            }
                                            _ => todo!(),
                                        }
                                    } else {
                                        todo!()
                                    }
                                } else {
                                    None
                                }
                            }
                            _ => todo!(),
                        }
                    } else {
                        todo!()
                    }
                } else {
                    None
                }
            } else {
                panic!("no type found");
            }
        }
        _ => todo!(),
    }
}