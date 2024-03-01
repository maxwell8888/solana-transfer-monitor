use solana_transaction_status::{EncodedTransaction, UiConfirmedBlock};

use crate::USDC_MINT_ADDRESS;

/// Get all successful USDC transactions
/// Do text search of data returned by get_block so we can verify the parsing functions are successfully accounting for all transactions involving USDC
pub fn get_all_successful_usdc_transactions(block: UiConfirmedBlock) -> Vec<String> {
    let mut transaction_signatures = Vec::new();
    if let Some(transactions) = block.transactions {
        for transaction in transactions {
            let debug_string = format!("{transaction:?}");

            if transaction.meta.unwrap().err.is_none() && debug_string.contains(USDC_MINT_ADDRESS)
            {
                let signature = match transaction.transaction {
                    EncodedTransaction::LegacyBinary(_) => todo!(),
                    EncodedTransaction::Binary(_, _) => todo!(),
                    EncodedTransaction::Json(ui_transaction) => ui_transaction.signatures,
                    EncodedTransaction::Accounts(_) => todo!(),
                };
                if let Some(signature) = signature.into_iter().next() {
                    transaction_signatures.push(signature);
                }
            }
        }
    } else {
        panic!("Error: no transactions found for block");
    }
    transaction_signatures
}
