use std::str::FromStr;

use neptune_cash::api::export::BlockHeight;
use neptune_cash::prelude::tasm_lib::prelude::Digest;
use neptune_cash::protocol::consensus::block::block_selector::BlockSelector;
use neptune_cash::protocol::consensus::block::block_selector::BlockSelectorParseError;
use serde::Deserialize;
use serde::Serialize;

/// represents either a block-height or a block digest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeightOrDigest {
    /// Identifies block by Digest (hash)
    Digest(Digest),
    /// Identifies block by Height (count from genesis)
    Height(BlockHeight),
}

impl std::fmt::Display for HeightOrDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Digest(d) => write!(f, "{d}"),
            Self::Height(h) => write!(f, "{h}"),
        }
    }
}

impl FromStr for HeightOrDigest {
    type Err = BlockSelectorParseError;

    // note: this parses the output of impl Display for HeightOrDigest
    // note: this is used by clap parser in neptune-cli for block-info command
    //       and probably future commands as well.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.parse::<u64>() {
            Ok(h) => Self::Height(h.into()),
            Err(_) => Self::Digest(Digest::try_from_hex(s)?),
        })
    }
}

impl From<HeightOrDigest> for BlockSelector {
    fn from(hd: HeightOrDigest) -> Self {
        match hd {
            HeightOrDigest::Height(h) => Self::Height(h),
            HeightOrDigest::Digest(d) => Self::Digest(d),
        }
    }
}
