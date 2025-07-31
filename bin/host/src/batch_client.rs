use std::ops::Range;

use alloy_primitives::hex::FromHex;
use eyre::eyre;
use eyre::Result;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use twine_rpc::TwineBatchApiClient;
use twine_types::BatchMeta;

pub struct BatchClient {
    inner: HttpClient,
}

#[allow(dead_code)]
impl BatchClient {
    pub fn new(url: &str) -> Self {
        let rpc_client = HttpClientBuilder::default().build(url).unwrap();
        Self { inner: rpc_client }
    }

    pub async fn get_latest_batch(&self) -> Result<u64> {
        self.inner.get_latest_batch().await.map_err(Into::into)
    }

    pub async fn get_full_batch(&self, batch: u64) -> Result<BatchMeta> {
        self.inner.get_full_batch(batch).await.map_err(Into::into)
    }

    pub async fn get_batch_hash(&self, batch: u64) -> Result<[u8; 32]> {
        let hash_hex = self
            .inner
            .get_batch_hash(batch)
            .await?
            .ok_or_else(|| eyre!("batch {batch} not found"))?;

        <[u8; 32]>::from_hex(hash_hex.trim_start_matches("0x"))
            .map_err(|_| eyre!("invalid batch-hash hex"))
    }

    pub async fn get_batch_number_for_block(&self, block: u64) -> Result<u64> {
        self.inner
            .get_batch_number_for_block(block)
            .await?
            .ok_or_else(|| eyre!("Block {block} not found"))
    }

    pub async fn get_blocks_in_batch(&self, batch: u64) -> Result<Range<u64>> {
        self.inner.get_blocks_in_batch(batch).await?.ok_or_else(|| eyre!("Batch {batch} not found"))
    }
}
