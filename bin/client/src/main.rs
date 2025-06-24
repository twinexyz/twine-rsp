#![no_main]
sp1_zkvm::entrypoint!(main);

use rsp_client_executor::{
    executor::{EthClientExecutor, DESERIALZE_INPUTS},
    io::{CommittedHeader, EthClientExecutorInput},
    utils::profile_report,
};
use std::sync::Arc;

pub fn main() {
    // Read the input.
    let inputs = profile_report!(DESERIALZE_INPUTS, {
        let input = sp1_zkvm::io::read_vec();
        serde_json::from_slice::<Vec<EthClientExecutorInput>>(&input).unwrap()
    });

    let mut headers = vec![];

    // Execute the block.
    for input in inputs {
        let executor = EthClientExecutor::eth(
            Arc::new((&input.genesis).try_into().unwrap()),
            input.custom_beneficiary,
        );
        let header = executor.execute(input).expect("failed to execute client");
        headers.push(header);
    }

    let committed_headers: Vec<CommittedHeader> = headers.into_iter().map(|header| {
        CommittedHeader {
            header,
        }
    }).collect(); 
    // Commit the block header.
    sp1_zkvm::io::commit::<Vec<CommittedHeader>>(&committed_headers.into());
}
