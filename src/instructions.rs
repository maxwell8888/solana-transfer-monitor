use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::{collections::HashMap, fmt, str::from_utf8};

use crate::USDC_MINT_ADDRESS;

pub fn handle_parsed_instruction(
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
