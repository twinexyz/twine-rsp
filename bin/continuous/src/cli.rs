use std::path::PathBuf;

use alloy_chains::Chain;
use alloy_provider::{network::AnyNetwork, Provider, RootProvider};
use clap::Parser;
use rsp_host_executor::Config;
use rsp_primitives::genesis::{genesis_from_json, Genesis};
use sp1_sdk::SP1ProofMode;
use url::Url;

/// The arguments for the cli.
#[derive(Debug, Clone, Parser)]
pub struct Args {
    /// The HTTP rpc url used to fetch data about the block.
    #[clap(long, env)]
    pub http_rpc_url: Url,

    #[clap(long, env)]
    pub chain_id: Option<u64>,


    #[clap(long, env)]
    pub genesis_path: Option<PathBuf>,

    #[clap(long, env)]
    pub prove: bool, 
    

    /// The WS rpc url used to fetch data about the block.
    #[clap(long, env)]
    pub ws_rpc_url: Url,

    /// The database connection string.
    #[clap(long, env)]
    pub database_url: String,

    /// The maximum number of concurrent executions.
    #[clap(long, env)]
    pub max_concurrent_executions: usize,

    /// Retry count on failed execution.
    #[clap(long, env, default_value_t = 3)]
    pub execution_retries: usize,

    /// PagerDuty integration key.
    #[clap(long, env)]
    pub pager_duty_integration_key: Option<String>,
}

impl Args {
     pub async fn as_config(&self) -> eyre::Result<Config> {
        // We don't need RPC when using cache with known chain ID, so we leave it as `Option<Url>`
        // here and decide on whether to panic later.
        //
        // On the other hand chain ID is always needed.
        let (rpc_url, chain_id) = match self.chain_id {
            Some(chain_id) => (Some(self.http_rpc_url.clone()), chain_id),
            None => {
                // We can find out about chain ID from RPC.
                let provider = RootProvider::<AnyNetwork>::new_http(self.http_rpc_url.clone());

                (Some(self.http_rpc_url.clone()), provider.get_chain_id().await?)
            }
        };

        let genesis = if let Some(genesis_path) = &self.genesis_path {
            // let genesis_json = fs::read_to_string(genesis_path)
            //     .map_err(|err| eyre::eyre!("Failed to read genesis file: {err}"))?;
            // let genesis = serde_json::from_str::<alloy_genesis::Genesis>(&genesis_json)?;

            let genesis = genesis_from_json(genesis_path.to_str().unwrap()).unwrap();

            Genesis::Custom(genesis.config)
        } else {
            chain_id.try_into()?
        };

        let chain = Chain::from_id(chain_id);

        let config = Config {
            chain,
            genesis,
            rpc_url,
            cache_dir: None,
            custom_beneficiary: None,
            prove_mode: self.prove.then_some(SP1ProofMode::Groth16),
            skip_client_execution: false,
            opcode_tracking: false,
        };

        Ok(config)
    }
}
