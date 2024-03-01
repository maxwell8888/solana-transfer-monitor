use pretty_assertions::assert_eq;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transfer_monitor::utils::get_all_successful_usdc_transactions;
use solana_transfer_monitor::{instructions::Transfer, make_block_config, write_block_transfers};
use std::io::Write;
use std::str::from_utf8;

#[rustfmt::skip]
const USDC_TRANSFER_FROM_250684537: [(&str, &str, &str); 33] = [
    // https://solscan.io/tx/3Tf9PsFsv3MDmr5UEviSGGkAwDXRbgYpN3vQrnSwgFHXayNk84fr8NwLZWS9qjhaEoXwXAu7SNa7vnY52KNSK7Cz
    ("GnRWVzHnXxqBLd2TwgVfe8o1KjPW8JuFCeXSMXQtNFP4", "49GkFii613smDpQbczLNjPt9cCsbJzUkXYYCAw432STd", "1,400"),
    ("83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d", "GnRWVzHnXxqBLd2TwgVfe8o1KjPW8JuFCeXSMXQtNFP4", "1,400.01"),

    // https://solscan.io/tx/3CLSS7DNxNxZjWYqwfTyF65LBZdWjFVSrvMC7cjy8zVE12g57CS9ruMGZ4iJnDa7cd7Ec8RpeQM1k1UnUZuX8qZN
    ("D71c54RUPqNFzWHPHTQ2v4bXdLUF2xhwerdGUsfRBSF2", "EXHyQxMSttcvLPwjENnXCPZ8GmLjJYHtNBnAkcFeFKMn", "70"),
    ("CHkBz1nNFSy3b5DjDf3AbJBZLPKUiwSVaoQ8qv6epXr5", "D71c54RUPqNFzWHPHTQ2v4bXdLUF2xhwerdGUsfRBSF2", "70"),

    // https://solscan.io/tx/VK8fbbeetxfT9dDxxtJR7cyPbQQd48VKQxSG3jBt9nXQeTXanXvGTWGhFecesSzhkUMkiUdYZmsN6dG4GGyTzX7
    ("83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d", "5aqJFL4rb53dEMNzT6zYGkoiMNnPZ6BpqeLaB57DcX1p", "1,118.99"),
    ("5aqJFL4rb53dEMNzT6zYGkoiMNnPZ6BpqeLaB57DcX1p", "82nEEkdjAf2TsVVj189DgRdp7kkQ9Ghs4LqY1gcgbjxn", "1,118.99"),

    // https://solscan.io/tx/2uFYxjq6bb3PJTF3oZdJmA3oeoZETtRT8duJEXbvaBM2bhKDXt8HVFPbK1GcnaiyuzWbXtVYkLoyyvSqm7JTyFxV
    ("vnuCkQAab1ZVRNi2pphJ1y3SsDBsf1br5VEzTv9a1nX", "4xDsmeTWPNjgSVSS1VTfzFq3iHZhp77ffPkAmkZkdu71", "222.68"),
    ("4xDsmeTWPNjgSVSS1VTfzFq3iHZhp77ffPkAmkZkdu71", "H6Vb6qdn4pfg1tmqXhVK8WQocsfeUWRhTNZFMjeypsRE", "222.68"),

    // https://solscan.io/tx/4g6FLoUERobLDNTrPP3Gd3qVM2W7cvaiWPR5ncyCjE48S85WHMxtb5bs3T26zXjipxmSdWk1s26ypRXxG797eZFs
    // This transaction has a second signature dMKVNDg72AYd8dpzM6AhhQ3AEDkqd124nNYLwHzig59U21eydBXwmjE5ZXUyrK8qhpw4HJf6TMyL3aEVcU8WsVG and the solscan page for that "tx" returns no data
    ("6H593doaHAjxWPUGNFdJY4ta5sQssSQo3Aj8LdgcTbFp", "AiM61XjxAwSK4KMn8ykM21sowfsTr95QFamdvx3aHgB4", "127.42"),
    ("6H593doaHAjxWPUGNFdJY4ta5sQssSQo3Aj8LdgcTbFp", "FudPMePeNqmnjMX19zEKDfGXpbp6HAdW6ZGprB5gYRTZ", "2.3403"),
    ("6H593doaHAjxWPUGNFdJY4ta5sQssSQo3Aj8LdgcTbFp", "JBGUGPmKUEHCpxGGoMowQxoV4c7HyqxEnyrznVPxftqk", "0.260044"),

    // https://solscan.io/tx/3EPNiMGpQasd1RTH7VjEqLFcAzPDdtMaQ7NBxzXKmFG13J8Mp9uXsNuQUTjNWWf7cdtfMFpvEarB4HWyXCqqiLYn
    ("CTz5UMLQm2SRWHzQnU62Pi4yJqbNGjgRBHqqp6oDHfF7", "AxjdyLn3FPWUtGSaH7CPopyABv1XPQMifB3Kp37JNhmE", "34,772.53"),
    ("AxjdyLn3FPWUtGSaH7CPopyABv1XPQMifB3Kp37JNhmE", "CTz5UMLQm2SRWHzQnU62Pi4yJqbNGjgRBHqqp6oDHfF7", "9,960.91"),
    ("AxjdyLn3FPWUtGSaH7CPopyABv1XPQMifB3Kp37JNhmE", "CTz5UMLQm2SRWHzQnU62Pi4yJqbNGjgRBHqqp6oDHfF7", "4,990.42"),
    ("AxjdyLn3FPWUtGSaH7CPopyABv1XPQMifB3Kp37JNhmE", "CTz5UMLQm2SRWHzQnU62Pi4yJqbNGjgRBHqqp6oDHfF7", "19,842.14"),

    // https://solscan.io/tx/26cWRv2fsS83rXYxSzvaRjv3wguKW9nd6snEofXgeLQGXbWveobnAw6hHYQxMmeVN1f8NuyzyhZmQBWwcAga9UDD
    // USDC appears in balance change table, but there is no change
    
    // https://solscan.io/tx/61cu2ckufuTed1kqxG5o6kHQgLMpVhD8YmkAmWx8uBDpRgZYuTugCxWfw1VB1H7KZoctRHJnkPZadrrbGtB7eh4B
    ("ASx1wk74GLZsxVrYiBkNKiViPLjnJQVGxKrudRgPir4A", "7BgkgTP2Qj5yLKrW3XB4oMba62mwRYqpr4bXEeasnyzt", "10.12"),
    ("7BgkgTP2Qj5yLKrW3XB4oMba62mwRYqpr4bXEeasnyzt", "ASx1wk74GLZsxVrYiBkNKiViPLjnJQVGxKrudRgPir4A", "8.2604"),
    
    // https://solscan.io/tx/3SjrSsUNpS3brENPmeZWKh9yVGiDtTnwU5SASfWenb6JY6525MG3Bk1P8LoUufjTaBVhqTxUCdFTmjzVPTWJ7HF1
    // USDC appears in balance change table, but there is no change
    
    // https://solscan.io/tx/H3SXgidvZe4VGDJ6iMwUZCSNhaAqkUgsXbTz4mhZ4qjtKeDNy3BRCN5xGNFRAfcT79kbfSj1k2ifJYCYBHiEQvx
    // USDC appears in balance change table, but there is no change
    
    // https://solscan.io/tx/3ybfFHjVESwAXv3pqeYb3eQcTXjKH7QE5V8RiL7H3uw5NVrqAEiQrKyoPvivQSacC9Dx9RCyax3oFTrx11ehdHj4
    ("7rhxnLV8C77o6d8oz26AgK8x8m5ePsdeRawjqvojbjnQ", "7KFK8aQdrqyxe7V1iuMsP6wRopmqnsDCwAGHwVWE7Tec", "4,711.51"),
    ("7KFK8aQdrqyxe7V1iuMsP6wRopmqnsDCwAGHwVWE7Tec", "7rhxnLV8C77o6d8oz26AgK8x8m5ePsdeRawjqvojbjnQ", "24,503.34"),
    ("7rhxnLV8C77o6d8oz26AgK8x8m5ePsdeRawjqvojbjnQ", "7KFK8aQdrqyxe7V1iuMsP6wRopmqnsDCwAGHwVWE7Tec", "20,814.13"),
    
    // https://solscan.io/tx/2FMfMVeLTYNmF5wLvQNgJSRwZizyfLhrQ23gfKc5XY7A29ArRE6kyqhMHGAAWNjXL7bQW4yS5jFX5zTYmzMbnPmK
    // USDC appears in balance change table, but there is no change

    // https://solscan.io/tx/4VFnM6HKxWVuv88nGPEUFmYQoGdjg9CMYgzF6ZSHqsR6kpQTookEQvE7gv2Vwi272w2WZwrJxg5rLPnQgHS5wvpQ
    ("5ZuR4supLRJ8eQvpqur2pfhNnjuu1guzaLbzeWv4bM7E", "DCAK36VfExkPdAkYUQg6ewgxyinvcEyPLyHjRbmveKFw", "1.0007"),
    ("DCAK36VfExkPdAkYUQg6ewgxyinvcEyPLyHjRbmveKFw", "9nnLbotNTcUhvbrsA6Mdkx45Sm82G35zo28AqUvjExn8", "1.0007"),
    // ("9nnLbotNTcUhvbrsA6Mdkx45Sm82G35zo28AqUvjExn8", "Raydium Authority V4", "1.0007"),
    ("9nnLbotNTcUhvbrsA6Mdkx45Sm82G35zo28AqUvjExn8", "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", "1.0007"),


    // https://solscan.io/tx/4LuZTqpLKLKQsGhnG1rG1C35Lj6iZQdopPoqCuhGr6Vm9LC9EEjKZzPeGXFmyYHCZ5cyuDPMQNjz1GSQwViVzpd2
    // ("Meteora DLMM (SOL-USDC) Pool", "9nnLbotNTcUhvbrsA6Mdkx45Sm82G35zo28AqUvjExn8", "1.3418"),
    ("FoSDw2L5DmTuQTFe55gWPDXf88euaxAEKFre74CnvQbX", "9nnLbotNTcUhvbrsA6Mdkx45Sm82G35zo28AqUvjExn8", "1.3418"),
    ("9nnLbotNTcUhvbrsA6Mdkx45Sm82G35zo28AqUvjExn8", "3ESUFCnRNgZ7Mn2mPPUMmXYaKU8jpnV9VtA17M7t2mHQ", "1.3418"),

    // https://solscan.io/tx/3y9wmVAzokZntkHZbgD51CsLkkwy6BNshwaPKkPqdCnT5qCBqhKLjLVvZu62j1WHX3PNrsHCxVE6VvARv368gfmP
    ("3YD74MctB2RCNGaMUYQRGrTdPUFh6ZMHjWau5f6Br8mR", "7rhxnLV8C77o6d8oz26AgK8x8m5ePsdeRawjqvojbjnQ", "1,989.48"),
    ("7rhxnLV8C77o6d8oz26AgK8x8m5ePsdeRawjqvojbjnQ", "3YD74MctB2RCNGaMUYQRGrTdPUFh6ZMHjWau5f6Br8mR", "1,989.22"),

    // https://solscan.io/tx/P8x7dYF67nKWAS9Y7saqZA4tkCqkTAJkZqEo3L5ipAuj1eHuCVA7ytMi6YArjS4Ku5g9J9SZTJGArBDXzu55iCc
    ("24EkAyBiM8Lwf7zVDiHshGGRtVGbx9PzF12FEWpWNy7t", "6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB", "100.39"),
    ("6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB", "J4uBbeoWpZE8fH58PM1Fp9n9K6f1aThyeVCyRdJbaXqt", "69.27"),
    // ("6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB", "Meteora DLMM (USDC-USDT) Pool", "31.12"),
    ("6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB", "ARwi1S4DaiTG5DX7S4M4ZsrXqpMD1MrTmbu9ue2tpmEq", "31.12"),

    // https://solscan.io/tx/4fTwCP9K9hz96FKCGuET96xtgP7VdaJyz37dNGkpdb85p3Cpf4H2EGtGAD48ApoMg43TBUssNthFeFyxc9MDK3rH
    ("5nM1CTQwKXFZo5yJYC8J1pgj32JW6Fx8DpQAtPZ8aiLw", "EXHyQxMSttcvLPwjENnXCPZ8GmLjJYHtNBnAkcFeFKMn", "12"),
    // ("Meteora DLMM (HNT-USDC) Pool", "5nM1CTQwKXFZo5yJYC8J1pgj32JW6Fx8DpQAtPZ8aiLw", "12"),
    ("1koYvNEJ5gWXZ6V3re8xwXDHrEpHf4vNYrNGv4bhrqK", "5nM1CTQwKXFZo5yJYC8J1pgj32JW6Fx8DpQAtPZ8aiLw", "12"),

    // https://solscan.io/tx/2UGj5V8Ry8ggY6scjRy6F1kVs6pnz1Lrtm1WGjXbWYJA8zVszmm3dU8ACu7ZwtKyHS34coqqLw7H3sQVob9Ctc9W
    ("2MNGqr5eStzyBSYMNTDPo47Z5sKqx5LyEbi9tGagDNzo", "6YJWm3nhHXGPvgAHErWcNmqPQtSSHZhvtmE4U9Adwb3g", "172.77"),

];

#[test]
fn transfers_for_block_250684537() {
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::finalized());

    // block slot with a smaller number of USDC transfers: 250684537
    let slot = 250684537;

    let block = client
        .get_block_with_config(slot, make_block_config())
        .unwrap();

    let mut buffer: Vec<u8> = Vec::new();
    write_block_transfers(block, slot, &mut buffer).unwrap();
    let actual = from_utf8(&buffer).unwrap();

    let mut test_cases_buffer: Vec<u8> = Vec::new();
    writeln!(&mut test_cases_buffer, "Latest block: {slot}").unwrap();
    for tx in USDC_TRANSFER_FROM_250684537 {
        let transfer = Transfer {
            source_owner: tx.0.to_string(),
            destination_owner: tx.1.to_string(),
            formatted_amount: tx.2.to_string(),
        };
        writeln!(&mut test_cases_buffer, "{transfer}").unwrap();
    }
    let expected = from_utf8(&test_cases_buffer).unwrap();

    assert_eq!(expected, actual);
}

#[rustfmt::skip]
const USDC_TRANSACTIONS_FROM_250684537: [&str; 18] = [
    "3Tf9PsFsv3MDmr5UEviSGGkAwDXRbgYpN3vQrnSwgFHXayNk84fr8NwLZWS9qjhaEoXwXAu7SNa7vnY52KNSK7Cz",
    "3CLSS7DNxNxZjWYqwfTyF65LBZdWjFVSrvMC7cjy8zVE12g57CS9ruMGZ4iJnDa7cd7Ec8RpeQM1k1UnUZuX8qZN",
    "VK8fbbeetxfT9dDxxtJR7cyPbQQd48VKQxSG3jBt9nXQeTXanXvGTWGhFecesSzhkUMkiUdYZmsN6dG4GGyTzX7",
    "2uFYxjq6bb3PJTF3oZdJmA3oeoZETtRT8duJEXbvaBM2bhKDXt8HVFPbK1GcnaiyuzWbXtVYkLoyyvSqm7JTyFxV",
    "4g6FLoUERobLDNTrPP3Gd3qVM2W7cvaiWPR5ncyCjE48S85WHMxtb5bs3T26zXjipxmSdWk1s26ypRXxG797eZFs",
    "3EPNiMGpQasd1RTH7VjEqLFcAzPDdtMaQ7NBxzXKmFG13J8Mp9uXsNuQUTjNWWf7cdtfMFpvEarB4HWyXCqqiLYn",
    "26cWRv2fsS83rXYxSzvaRjv3wguKW9nd6snEofXgeLQGXbWveobnAw6hHYQxMmeVN1f8NuyzyhZmQBWwcAga9UDD", // *
    "61cu2ckufuTed1kqxG5o6kHQgLMpVhD8YmkAmWx8uBDpRgZYuTugCxWfw1VB1H7KZoctRHJnkPZadrrbGtB7eh4B",
    "3SjrSsUNpS3brENPmeZWKh9yVGiDtTnwU5SASfWenb6JY6525MG3Bk1P8LoUufjTaBVhqTxUCdFTmjzVPTWJ7HF1", // *
    "H3SXgidvZe4VGDJ6iMwUZCSNhaAqkUgsXbTz4mhZ4qjtKeDNy3BRCN5xGNFRAfcT79kbfSj1k2ifJYCYBHiEQvx", // *
    "3ybfFHjVESwAXv3pqeYb3eQcTXjKH7QE5V8RiL7H3uw5NVrqAEiQrKyoPvivQSacC9Dx9RCyax3oFTrx11ehdHj4",
    "2FMfMVeLTYNmF5wLvQNgJSRwZizyfLhrQ23gfKc5XY7A29ArRE6kyqhMHGAAWNjXL7bQW4yS5jFX5zTYmzMbnPmK", // *
    "4VFnM6HKxWVuv88nGPEUFmYQoGdjg9CMYgzF6ZSHqsR6kpQTookEQvE7gv2Vwi272w2WZwrJxg5rLPnQgHS5wvpQ",
    "4LuZTqpLKLKQsGhnG1rG1C35Lj6iZQdopPoqCuhGr6Vm9LC9EEjKZzPeGXFmyYHCZ5cyuDPMQNjz1GSQwViVzpd2",
    "3y9wmVAzokZntkHZbgD51CsLkkwy6BNshwaPKkPqdCnT5qCBqhKLjLVvZu62j1WHX3PNrsHCxVE6VvARv368gfmP",
    "P8x7dYF67nKWAS9Y7saqZA4tkCqkTAJkZqEo3L5ipAuj1eHuCVA7ytMi6YArjS4Ku5g9J9SZTJGArBDXzu55iCc",
    "4fTwCP9K9hz96FKCGuET96xtgP7VdaJyz37dNGkpdb85p3Cpf4H2EGtGAD48ApoMg43TBUssNthFeFyxc9MDK3rH",
    "2UGj5V8Ry8ggY6scjRy6F1kVs6pnz1Lrtm1WGjXbWYJA8zVszmm3dU8ACu7ZwtKyHS34coqqLw7H3sQVob9Ctc9W",
];
// * USDC appears in balance change table, but there is no change in balance and no instructions involving USDC so these transactions don't log anything

#[test]
fn get_text_search_of_data() {
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::finalized());

    let slot = 250684537;

    let block = client
        .get_block_with_config(slot, make_block_config())
        .unwrap();

    let text_search_transactions = get_all_successful_usdc_transactions(block);

    assert_eq!(
        USDC_TRANSACTIONS_FROM_250684537,
        text_search_transactions.as_slice()
    );
}
