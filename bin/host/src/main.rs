#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use std::{collections::HashMap, sync::Arc};

use clap::Parser;
use execute::PersistExecutionReport;
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

mod execute;

mod cli;
use cli::HostArgs;

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

        executor.execute(block_number, args.to_block.unwrap_or(block_number), HashMap::new()).await?; // TODO: load validator set here if necessary
    } else {
        let elf = include_elf!("rsp-client").to_vec();
        let validator_sets = HashMap::new();
        let block_execution_strategy_factory =
            create_eth_block_execution_strategy_factory(&config.genesis, config.custom_beneficiary, validator_sets.clone());
        let provider = config.rpc_url.as_ref().map(|url| create_provider(url.clone()));

        let executor = build_executor::<EthExecutorComponents<_>, _>(
            elf,
            provider,
            block_execution_strategy_factory,
            prover_client,
            persist_execution_report,
            config,
        )
        .await?;

        executor.execute(block_number, args.to_block.unwrap_or(block_number), validator_sets.clone()).await?;
    }

    Ok(())
}

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
