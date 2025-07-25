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
    pub from_block: u64,
    pub to_block: u64,
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
        uint64 from_block;
        uint64 to_block;
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

    pub fn abi_decode_packed(public_commitment: Vec<u8>) -> Result<Self, String> {
        if public_commitment.len() != 64 {
            return Err("invalid length".to_string());
        }
        let from_block_byte: [u8; 8] = public_commitment[0..8].try_into().unwrap_or([0u8; 8]);
        let from_block = u64::from_be_bytes(from_block_byte);

        let to_block_byte: [u8; 8] = public_commitment[8..16].try_into().unwrap_or([0u8; 8]);
        let to_block = u64::from_be_bytes(to_block_byte);

        let batch_hash = FixedBytes::<32>::from_slice(&public_commitment[16..48]);

        let ethereum_message_count =
            u64::from_be_bytes(public_commitment[48..56].try_into().unwrap_or_default());
        let solana_message_count =
            u64::from_be_bytes(public_commitment[56..64].try_into().unwrap_or_default());

        Ok(Self { from_block, to_block, batch_hash, ethereum_message_count, solana_message_count })
    }
}

impl From<PublicCommitment> for SolPublicCommitment {
    fn from(value: PublicCommitment) -> Self {
        Self {
            from_block: value.from_block,
            to_block: value.to_block,
            batch_hash: value.batch_hash,
            ethereum_message_count: value.ethereum_message_count,
            solana_message_count: value.solana_message_count,
        }
    }
}
