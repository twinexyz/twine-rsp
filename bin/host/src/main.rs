#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use std::{collections::HashMap, env, sync::Arc};

use alloy_primitives::keccak256;
use alloy_primitives::U256;
use alloy_provider::Provider;
use clap::Parser;
use execute::PersistExecutionReport;
use eyre::ensure;
use reth_trie_common::AccountProof;
use rsp_client_executor::io::BatchMetadata;
use rsp_host_executor::{
    build_executor, create_eth_block_execution_strategy_factory,
    create_op_block_execution_strategy_factory, BlockExecutor, EthExecutorComponents,
    OpExecutorComponents,
};
use rsp_provider::create_provider;
use sp1_sdk::{include_elf, EnvProver};
use tracing_subscriber::{
    filter::EnvFilter, fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

mod batch_client;
mod cli;
mod execute;
use cli::HostArgs;
use twine_constants::precompiles::TWINE_SYSTEM_STORAGE_CONTRACT;

use crate::batch_client::BatchClient;
// TODO: After consensus precompile is merged
// use twine_constants::chains::RECOGNIZED_CHAINS;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize the environment variables.
    dotenv::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    // Initialize the logger.
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive("sp1_core_machine=warn".parse().unwrap())
                .add_directive("sp1_core_executor::executor=warn".parse().unwrap())
                .add_directive("sp1_prover=warn".parse().unwrap()),
        )
        .init();

    // Parse the command line arguments.
    let args = HostArgs::parse();
    let block_number = args.block_number;
    let report_path = args.report_path.clone();
    let config = args.as_config().await?;
    let persist_execution_report = PersistExecutionReport::new(
        config.chain.id(),
        report_path,
        args.precompile_tracking,
        args.opcode_tracking,
    );

    let prover_client = Arc::new(EnvProver::new());

    if config.chain.is_optimism() {
        let elf = include_elf!("rsp-client-op").to_vec();
        let block_execution_strategy_factory =
            create_op_block_execution_strategy_factory(&config.genesis);
        let provider = config.rpc_url.as_ref().map(|url| create_provider(url.clone()));

        let executor = build_executor::<OpExecutorComponents<_>, _>(
            elf,
            provider,
            block_execution_strategy_factory,
            prover_client,
            persist_execution_report,
            config,
        )
        .await?;

        executor
            .execute(block_number, args.to_block.unwrap_or(block_number), None, HashMap::new())
            .await?; // TODO: load validator set here if necessary
    } else {
        let elf = include_elf!("rsp-client").to_vec();
        // TODO: After consensus precompile is merged
        // let validator_sets = load_validator_sets();
        let validator_sets = HashMap::new();
        let block_execution_strategy_factory = create_eth_block_execution_strategy_factory(
            &config.genesis,
            config.custom_beneficiary,
            validator_sets.clone(),
        );
        let provider = config.rpc_url.as_ref().map(|url| create_provider(url.clone()));

        let batch_metadata = match provider {
            Some(ref prov) => {
                let to_block = args.to_block.unwrap_or(block_number);
                build_batch_metadata(
                    prov,
                    config.rpc_url.as_ref().unwrap().as_str(),
                    block_number,
                    to_block,
                )
                .await?
            }
            None => None,
        };
        let executor = build_executor::<EthExecutorComponents<_>, _>(
            elf,
            provider,
            block_execution_strategy_factory,
            prover_client,
            persist_execution_report,
            config,
        )
        .await?;

        executor
            .execute(
                block_number,
                args.to_block.unwrap_or(block_number),
                batch_metadata,
                validator_sets.clone(),
            )
            .await?;
    }

    Ok(())
}

// TODO: After consensus precompile is merged
// fn load_validator_sets() -> HashMap<String, String> {
//     let validator_set_base_path = env::var("L1_VALIDATOR_SET_PATH").expect("provide the base directory path that contains the validator set files for the chains you want to register in the precompiles");
//     let validator_set_files = fs::read_dir(validator_set_base_path).unwrap();
//     let mut validator_set_hashmap = HashMap::new();
//     () = validator_set_files
//         .into_iter()
//         .map(|file| {
//             let file = file.unwrap();
//             let file_name = file.file_name().to_str().unwrap().to_string();
//             let splitted_name: Vec<&str> = file_name.split(".").collect();
//             if RECOGNIZED_CHAINS.contains(&splitted_name[0]) {
//                 let validator_set = fs::read_to_string(file.path()).unwrap();
//                 validator_set_hashmap.insert(splitted_name[0].to_string(), validator_set);
//             }
//         })
//         .collect();
//     validator_set_hashmap
// }

fn calculate_one_level_mapping_slot(inner_key: U256) -> U256 {
    // l1MessageExecutedCount is at 2nd index
    let mapping_slot = U256::from(2);

    let mut encoded = vec![];
    encoded.extend_from_slice(&inner_key.to_be_bytes_vec());
    encoded.extend_from_slice(&mapping_slot.to_be_bytes_vec());

    U256::from_be_slice(keccak256(&encoded).as_ref())
}

async fn build_batch_metadata(
    provider: &alloy_provider::RootProvider,
    rpc_url: &str,
    from_block: u64,
    to_block: u64,
) -> eyre::Result<Option<BatchMetadata>> {
    let batch_client = BatchClient::new(rpc_url);

    // Ensure same batch
    let from_batch = batch_client.get_batch_number_for_block(from_block).await?;
    let to_batch = batch_client.get_batch_number_for_block(to_block).await?;
    ensure!(from_batch == to_batch, "from/to blocks belong to different batches");

    // Previous batch hash (0x00…00 for genesis)
    let prev_batch_hash = if from_batch == 0 {
        [0u8; 32]
    } else {
        batch_client.get_batch_hash(from_batch - 1).await?
    };

    // Storage proofs
    let ethereum_slot = calculate_one_level_mapping_slot(U256::from(17000));
    let solana_slot = calculate_one_level_mapping_slot(U256::from(900));

    let proof = provider
        .get_proof(TWINE_SYSTEM_STORAGE_CONTRACT, vec![ethereum_slot.into(), solana_slot.into()])
        .block_id(to_block.into())
        .await?;

    let state_proofs = AccountProof::from_eip1186_proof(proof);

    Ok(Some(BatchMetadata { prev_batch_hash, state_proofs }))
}
