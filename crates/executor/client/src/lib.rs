#![cfg_attr(not(test), warn(unused_crate_dependencies))]

/// Client program input data types.
pub mod io;
#[macro_use]
pub mod utils;
pub mod custom;
pub mod error;
pub mod executor;
pub mod tracking;

mod into_primitives;
use alloy_sol_types::{sol, SolValue};
pub use into_primitives::{BlockValidator, FromInput, IntoInput, IntoPrimitives};
use revm_primitives::FixedBytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicCommitment {
    pub prev_batch_hash: FixedBytes<32>,
    pub batch_hash: FixedBytes<32>,
    pub ethereum_message_count: u64,
    pub solana_message_count: u64,
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub previous_block: FixedBytes<32>,
    pub block_hash: FixedBytes<32>,
    pub transaction_root: FixedBytes<32>,
    pub receipt_root: FixedBytes<32>,
}

sol! {
    struct SolBlockInfo {
       bytes32 previous_block;
       bytes32 block_hash;
       bytes32 transaction_root;
       bytes32 receipt_root;
    }

    struct SolPublicCommitment {
        bytes32 prev_batch_hash;
        bytes32 batch_hash;
        uint64 ethereum_message_count;
        uint64 solana_message_count;
    }
}

impl BlockInfo {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let sol_block_info = SolBlockInfo::from(self.clone());
        sol_block_info.abi_encode_packed()
    }
}

impl From<BlockInfo> for SolBlockInfo {
    fn from(value: BlockInfo) -> Self {
        Self {
            previous_block: value.previous_block,
            block_hash: value.block_hash,
            transaction_root: value.transaction_root,
            receipt_root: value.receipt_root,
        }
    }
}

impl PublicCommitment {
    pub fn abi_encode_packed(&self) -> Vec<u8> {
        let sol_pub_commitment = SolPublicCommitment::from(self.clone());
        sol_pub_commitment.abi_encode_packed()
    }

    pub fn abi_decode_packed(bytes: Vec<u8>) -> Result<Self, String> {
        const LEN: usize = 32 + 32 + 8 + 8; // 80 bytes
        if bytes.len() != LEN {
            return Err(format!("expected {} bytes, got {}", LEN, bytes.len()));
        }
    
        let mut idx = 0;
    
        let prev_batch_hash = FixedBytes::<32>::from_slice(&bytes[idx..idx + 32]);
        idx += 32;
    
        let batch_hash = FixedBytes::<32>::from_slice(&bytes[idx..idx + 32]);
        idx += 32;
    
        let ethereum_message_count = u64::from_be_bytes(bytes[idx..idx + 8].try_into().unwrap());
        idx += 8;
    
        let solana_message_count = u64::from_be_bytes(bytes[idx..idx + 8].try_into().unwrap());
        idx += 8;
    
        debug_assert_eq!(idx, LEN);
    
        Ok(Self {
            prev_batch_hash,
            batch_hash,
            ethereum_message_count,
            solana_message_count,
        })
    }
}

impl From<PublicCommitment> for SolPublicCommitment {
    fn from(value: PublicCommitment) -> Self {
        Self {
            prev_batch_hash: value.prev_batch_hash,
            batch_hash: value.batch_hash,
            ethereum_message_count: value.ethereum_message_count,
            solana_message_count: value.solana_message_count,
        }
    }
}
