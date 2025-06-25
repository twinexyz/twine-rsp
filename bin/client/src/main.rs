#![no_main]
sp1_zkvm::entrypoint!(main);

use rsp_client_executor::{
    executor::{EthClientExecutor, DESERIALZE_INPUTS},
    io::{CommittedHeader, EthClientExecutorInput},
    utils::profile_report,
    BlockInfo, PublicCommitment
};
use revm_primitives::{FixedBytes, keccak256};
use std::sync::Arc;
use std::collections::HashMap;

pub fn main() {
    // Read the input.

    let inputs = profile_report!(DESERIALZE_INPUTS, {
        let input: Vec<u8> = sp1_zkvm::io::read();
        serde_json::from_slice::<Vec<EthClientExecutorInput>>(&input).unwrap()
    });

    let validator_set_slice: Vec<u8> = sp1_zkvm::io::read();
    let validator_sets: HashMap<String, String> = serde_json::from_slice(&validator_set_slice).unwrap();


    let mut headers = vec![];

    // Execute the block.
    for input in inputs {
        let executor = EthClientExecutor::eth(
            Arc::new((&input.genesis).try_into().unwrap()),
            input.custom_beneficiary,
            validator_sets.clone(),
        );
        let header = executor.execute(input).expect("failed to execute client");
        headers.push(header);
    }

    let mut pub_commitment_slice = vec![];

    () = headers.clone()
        .iter()
        .map(|header| {
            let public_commitment = BlockInfo {
                previous_block: FixedBytes::from_slice(&header.parent_hash.0),
                block_hash: FixedBytes::from_slice(&header.hash_slow().0),
                transaction_root: FixedBytes::from_slice(&header.transactions_root.0),
                receipt_root: FixedBytes::from_slice(&header.receipts_root.0),
            };
            let mut public_commitment = public_commitment.abi_encode_packed();
            pub_commitment_slice.append(&mut public_commitment);
        })
        .collect();
    
    let public_commitment =
        PublicCommitment { from_block: headers.first().unwrap().number, to_block: headers.last().unwrap().number, batch_hash: keccak256(pub_commitment_slice) };

    // Commit the block header.
    sp1_zkvm::io::commit_slice(&public_commitment.abi_encode_packed());
}
