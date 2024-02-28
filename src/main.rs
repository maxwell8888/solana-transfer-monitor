use anyhow::Result;
use serde_json::Value;
use solana_client::{rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransaction, TransactionDetails, UiInstruction,
    UiMessage, UiParsedInstruction, UiTransactionEncoding,
};
use std::collections::HashMap;

// TODO NOTE: "The program should begin tracking from the latest block" note latest *block*, not slot
// TODO graceful shutdown
// TODO add logic to prevent exceeding rate limit, ie count number of requests every 10 seconds
// TODO https://solscan.io/tx/3ybfF... seems to just use one sig from the transaction, but the API provides a Vec... when can there be more than 1 sig?
// TODO combine transfers between the same accounts in the same direction in the same transaction?
// TODO techinically we don't need to keep asking for slot numbers, we can just calculate them ourselves and save 1 request for the rate limiting

// USDC
const TOKEN_MINT_ADDRESS: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

// CROWN
// const TOKEN_MINT_ADDRESS: &str = "GDfnEsia2WLAW5t8yx2X5j2mkfA74i5kwGdDuZHt7XmG";

// KIN
// const TOKEN_MINT_ADDRESS: &str = "kinXdEcpDQeHPEuQnqmUgtYykqKGVFq6CeVX5iAHJq6";

async fn run() -> Result<()> {
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    // let rpc_url = "https://api.devnet.solana.com".to_string();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::finalized());

    let _current_slot = client.get_slot().unwrap();
    let mut rpc_block_config = RpcBlockConfig::default();
    rpc_block_config.encoding = Some(UiTransactionEncoding::JsonParsed);
    rpc_block_config.transaction_details = Some(TransactionDetails::Full);
    rpc_block_config.max_supported_transaction_version = Some(0);

    // block slot with lots of USDC transfers: 250655260
    let _slot = 250655260;

    // block slot with a few USDC transfers: 250684537
    let slot = 250684537;

    let block = client.get_block_with_config(slot, rpc_block_config)?;

    println!("Latest block: {slot}");

    if let Some(transactions) = block.transactions {
        for transaction in transactions {
            let signature = match &transaction.transaction {
                EncodedTransaction::LegacyBinary(_) => todo!(),
                EncodedTransaction::Binary(_, _) => todo!(),
                EncodedTransaction::Json(ui_transaction) => ui_transaction.signatures.clone(),
                EncodedTransaction::Accounts(_) => todo!(),
            };

            // let my_sig = "3ybfFHjVESwAXv3pqeYb3eQcTXjKH7QE5V8RiL7H3uw5NVrqAEiQrKyoPvivQSacC9Dx9RCyax3oFTrx11ehdHj4";
            let my_sig = "3y9wmVAzokZntkHZbgD51CsLkkwy6BNshwaPKkPqdCnT5qCBqhKLjLVvZu62j1WHX3PNrsHCxVE6VvARv368gfmP";
            // let my_sig = "4g6FLoUERobLDNTrPP3Gd3qVM2W7cvaiWPR5ncyCjE48S85WHMxtb5bs3T26zXjipxmSdWk1s26ypRXxG797eZFs";
            // let my_sig = "3EPNiMGpQasd1RTH7VjEqLFcAzPDdtMaQ7NBxzXKmFG13J8Mp9uXsNuQUTjNWWf7cdtfMFpvEarB4HWyXCqqiLYn";
            // let my_sig = "4g6FLoUERobLDNTrPP3Gd3qVM2W7cvaiWPR5ncyCjE48S85WHMxtb5bs3T26zXjipxmSdWk1s26ypRXxG797eZFs";
            // let my_sig = "4fTwCP9K9hz96FKCGuET96xtgP7VdaJyz37dNGkpdb85p3Cpf4H2EGtGAD48ApoMg43TBUssNthFeFyxc9MDK3rH";
            // let my_sig = "61cu2ckufuTed1kqxG5o6kHQgLMpVhD8YmkAmWx8uBDpRgZYuTugCxWfw1VB1H7KZoctRHJnkPZadrrbGtB7eh4B";
            // let my_sig = "3Tf9PsFsv3MDmr5UEviSGGkAwDXRbgYpN3vQrnSwgFHXayNk84fr8NwLZWS9qjhaEoXwXAu7SNa7vnY52KNSK7Cz";

            // let my_sig = "3CLSS7DNxNxZjWYqwfTyF65LBZdWjFVSrvMC7cjy8zVE12g57CS9ruMGZ4iJnDa7cd7Ec8RpeQM1k1UnUZuX8qZN";

            if signature.first().unwrap() == my_sig {
                let mut accounts_map = HashMap::new();

                let parsed_accounts = match transaction.transaction {
                    EncodedTransaction::LegacyBinary(_) => todo!(),
                    EncodedTransaction::Binary(_, _) => todo!(),
                    EncodedTransaction::Json(ui_transaction) => match ui_transaction.message {
                        UiMessage::Parsed(ui_parsed_message) => ui_parsed_message.account_keys,
                        UiMessage::Raw(_) => {
                            todo!()
                        }
                    },
                    EncodedTransaction::Accounts(_) => todo!(),
                };

                if let Some(meta) = transaction.meta {
                    if meta.err.is_none() {
                        match meta.pre_token_balances {
                            OptionSerializer::Some(ui_transaction_token_balances) => {
                                for pre_ui_transaction_token_balance in
                                    ui_transaction_token_balances
                                {
                                    // TODO only add USDC to map
                                    // if pre_ui_transaction_token_balance.mint == TOKEN_MINT_ADDRESS {
                                    let pub_key = parsed_accounts
                                        [pre_ui_transaction_token_balance.account_index as usize]
                                        .pubkey
                                        .clone();
                                    let owner = match pre_ui_transaction_token_balance.owner {
                                        OptionSerializer::Some(owner) => owner,
                                        OptionSerializer::None => todo!(),
                                        OptionSerializer::Skip => todo!(),
                                    };
                                    accounts_map.insert(
                                        pub_key,
                                        (owner, pre_ui_transaction_token_balance.mint),
                                    );
                                    // }
                                }
                            }
                            OptionSerializer::None => todo!(),
                            OptionSerializer::Skip => todo!(),
                        }

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
                                                    UiParsedInstruction::Parsed(
                                                        parsed_instruction,
                                                    ) => {
                                                        if parsed_instruction.program == "spl-token"
                                                        {
                                                            handle_parsed_instruction(
                                                                &parsed_instruction.parsed,
                                                                &accounts_map,
                                                            )
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
            }
        }
    } else {
        eprintln!("Error: not transactions found for block in slot {slot}");
    }

    Ok(())
}

fn handle_parsed_instruction(
    parsed_instruction: &Value,
    accounts_map: &HashMap<String, (String, String)>,
) {
    match parsed_instruction {
        Value::Object(map) => {
            if let Some(type_) = map.get("type") {
                if type_ == "transfer" {
                    if let Some(info) = map.get("info") {
                        match info {
                            Value::Object(info_map) => {
                                let (source_owner, source_mint) = if let Some(source) =
                                    info_map.get("source")
                                {
                                    match source {
                                        Value::String(source) => accounts_map.get(source).unwrap(),
                                        _ => todo!(),
                                    }
                                } else {
                                    todo!()
                                };
                                let (destination_owner, destination_mint) =
                                    if let Some(destination) = info_map.get("destination") {
                                        match destination {
                                            Value::String(destination) => {
                                                accounts_map.get(destination).unwrap()
                                            }
                                            _ => todo!(),
                                        }
                                    } else {
                                        todo!()
                                    };
                                let amount = if let Some(amount) = info_map.get("amount") {
                                    match amount {
                                        Value::String(amount) => &amount[..amount.len() - 4],
                                        _ => todo!(),
                                    }
                                } else {
                                    todo!()
                                };
                                if source_mint != destination_mint {
                                    panic!("source and destination mints do no match");
                                } else if source_mint == TOKEN_MINT_ADDRESS {
                                    println!("TX detected: {source_owner} sent {amount} USDC to {destination_owner}")
                                }
                            }
                            _ => todo!(),
                        }
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
                                    // println!("TX detected: {source_owner} sent {amount} USDC to {destination_owner}")
                                    println!("transferChecked");
                                }
                            }
                            _ => todo!(),
                        }
                    }
                } else {
                    panic!()
                }
            } else {
                panic!("no type found");
            }
        }
        _ => todo!(),
        // serde_json::Value::Null => {}
        // _ => {}
    }
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => {}
        Err(e) => eprintln!("{e}"),
    }
}
