#![no_main]
sp1_zkvm::entrypoint!(main);

use reth_ethereum_primitives::EthPrimitives;
use revm_primitives::FixedBytes;
use rsp_client_executor::io::ClientInput;
use rsp_client_executor::{
    executor::{EthClientExecutor, DESERIALZE_INPUTS},
    utils::profile_report,
    PublicCommitment,
};
use twine_types::compute_batch_hash;
use twine_utils::merkle_root;
use std::sync::Arc;

pub fn main() {
    // Read the input.

    let client_inputs = profile_report!(DESERIALZE_INPUTS, {
        let input: Vec<u8> = sp1_zkvm::io::read();
        serde_json::from_slice::<ClientInput<EthPrimitives>>(&input).unwrap()
    });

    let ClientInput {
        client_input: inputs,
        batch_metadata,
        validator_sets,
    } = client_inputs;

    let mut headers = Vec::with_capacity(inputs.len());

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

    let first_header = headers.first().unwrap();
    let last_header  = headers.last().unwrap();

    let (prev_batch_hash, eth_msgs, sol_msgs) = match batch_metadata {
        Some(meta) => {
            // Verify account & storage proofs against latest state root
            meta.state_proofs.verify(last_header.state_root).expect("Failed to verify proofs");

            let mut eth_msgs = 0;
            let mut sol_msgs = 0;

            for (idx, storage) in meta.state_proofs.storage_proofs.iter().enumerate() {
                match idx {
                    0 => eth_msgs = storage.value.to::<u64>(),
                    1 => sol_msgs = storage.value.to::<u64>(),
                    _ => {}
                }
            }

            (FixedBytes::from(meta.prev_batch_hash), eth_msgs, sol_msgs)
        }
        None => (FixedBytes::ZERO, 0, 0), 
    };

    let state_roots: Vec<[u8; 32]> = headers
        .iter()
        .map(|h| h.state_root.0)
        .collect();

    let state_merkle_root = merkle_root(&state_roots);
    let batch_hash = compute_batch_hash(state_merkle_root, prev_batch_hash.into());

    let public_commitment = PublicCommitment {
        from_block: first_header.number,
        to_block:   last_header.number,
        prev_batch_hash,
        batch_hash,
        ethereum_message_count: eth_msgs,
        solana_message_count:   sol_msgs,
    };

    // Commit the block header.
    sp1_zkvm::io::commit_slice(&public_commitment.abi_encode_packed());
}
