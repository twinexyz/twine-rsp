#![no_main]
sp1_zkvm::entrypoint!(main);

use reth_ethereum_primitives::EthPrimitives;
use rsp_client_executor::io::ClientInput;
use rsp_client_executor::{
    executor::{EthClientExecutor, DESERIALZE_INPUTS},
    utils::profile_report,
    PublicCommitment,
};
use twine_types::compute_batch_hash;
use twine_utils::merkle_root;
use std::collections::HashMap;
use std::sync::Arc;

pub fn main() {
    // Read the input.

    let client_inputs = profile_report!(DESERIALZE_INPUTS, {
        let input: Vec<u8> = sp1_zkvm::io::read();
        serde_json::from_slice::<ClientInput<EthPrimitives>>(&input).unwrap()
    });

    let inputs = client_inputs.client_input;
    let batch_metadata = client_inputs.batch_metadata;
    let validator_sets: HashMap<String, String> = client_inputs.validator_sets;

    let mut headers = vec![];
    let mut prev_batch_hash = None;

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

    let mut ethereum_executed_txns_count = 0;
    let mut solana_executed_txns_count = 0;
    if let Some(batch_meta) = batch_metadata{
        let account_proof = batch_meta.state_proofs;
        let hdr = headers.last().unwrap();
        let state_root = hdr.state_root;
        account_proof.verify(state_root).expect("Failed to verify proofs");
        prev_batch_hash = Some(batch_meta.prev_batch_hash.into());

        // We verify proof of 2 storage slots
        // first slot: ethereum last message executed
        // second slot: solana last message executed
        for (i, storage) in account_proof.storage_proofs.iter().enumerate() {
            let val = storage.value.to::<u64>();
            match i {
                0 => ethereum_executed_txns_count = val,
                1 => solana_executed_txns_count = val,
                _ => {}
            }
        }
    }

    let mut pub_commitment_slice: Vec<[u8; 32]> = Vec::new();

    () = headers
        .clone()
        .iter()
        .map(|header| {
            let state_root = header.state_root.0;
            pub_commitment_slice.push(state_root);
        })
        .collect();

    let state_merkle_root = merkle_root(&pub_commitment_slice);
    let batch_hash = compute_batch_hash(state_merkle_root, prev_batch_hash);

    let public_commitment = PublicCommitment {
        from_block: headers.first().unwrap().number,
        to_block: headers.last().unwrap().number,
        batch_hash,
        ethereum_message_count: ethereum_executed_txns_count,
        solana_message_count: solana_executed_txns_count,
    };

    // Commit the block header.
    sp1_zkvm::io::commit_slice(&public_commitment.abi_encode_packed());
}
