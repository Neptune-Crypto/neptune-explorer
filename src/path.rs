use std::fmt;

use neptune_cash::api::export::Digest;
use neptune_cash::prelude::triton_vm::prelude::BFieldElement;

#[derive(Debug, Clone)]
pub enum ExplorerPath {
    Block(BlockIdentifier),
    Utxo(u64),
    Announcement(BlockIdentifier, usize),
    Rpc(RpcPath),
}

impl rand::distr::Distribution<ExplorerPath> for rand::distr::StandardUniform {
    fn sample<R: rand::Rng + ?std::marker::Sized>(&self, rng: &mut R) -> ExplorerPath {
        let _ = rng;
        match rng.random_range(0..4) {
            0 => ExplorerPath::Block(rng.random()),
            1 => ExplorerPath::Utxo(rng.random()),
            2 => ExplorerPath::Announcement(rng.random(), rng.random_range(0usize..(1 << 31))),
            3 => ExplorerPath::Rpc(rng.random()),
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for ExplorerPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ExplorerPath::Block(block_identifier) => format!("block/{block_identifier}"),
            ExplorerPath::Utxo(index) => format!("utxo/{index}"),
            ExplorerPath::Announcement(block_identifier, index) => {
                format!("announcement/{block_identifier}/{index}")
            }
            ExplorerPath::Rpc(rpc_path) => format!("rpc/{rpc_path}"),
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub enum BlockIdentifier {
    Tip,
    Genesis,
    Height(u64),
    Digest(Digest),
}
impl rand::distr::Distribution<BlockIdentifier> for rand::distr::StandardUniform {
    fn sample<R: rand::Rng + ?std::marker::Sized>(&self, rng: &mut R) -> BlockIdentifier {
        match rng.random_range(0..4) {
            0 => BlockIdentifier::Tip,
            1 => BlockIdentifier::Genesis,
            2 => BlockIdentifier::Height(rng.random_range(0u64..BFieldElement::P)),
            3 => BlockIdentifier::Digest(rng.random()),
            _ => unreachable!(),
        }
    }
}
impl fmt::Display for BlockIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BlockIdentifier::Tip => "tip".to_string(),
            BlockIdentifier::Genesis => "genesis".to_string(),
            BlockIdentifier::Height(height) => format!("height_or_digest/{height}"),
            BlockIdentifier::Digest(digest) => format!("height_or_digest/{digest:x}"),
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub enum RpcPath {
    BlockInfo(BlockIdentifier),
    BlockDigest(BlockIdentifier),
    UtxoDigest(usize),
}
impl rand::distr::Distribution<RpcPath> for rand::distr::StandardUniform {
    fn sample<R: rand::Rng + ?std::marker::Sized>(&self, rng: &mut R) -> RpcPath {
        match rng.random_range(0..3) {
            0 => RpcPath::BlockInfo(rng.random()),
            1 => RpcPath::BlockDigest(rng.random()),
            2 => RpcPath::UtxoDigest(rng.random_range(0usize..(1 << 31))),
            _ => unreachable!(),
        }
    }
}
impl fmt::Display for RpcPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RpcPath::BlockInfo(block_identifier) => format!("block_info/{block_identifier}"),
            RpcPath::BlockDigest(block_identifier) => format!("block_digest/{block_identifier}"),
            RpcPath::UtxoDigest(index) => format!("utxo_digest/{index}"),
        };
        write!(f, "{s}")
    }
}
