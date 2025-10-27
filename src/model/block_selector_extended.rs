use std::num::ParseIntError;
use std::str::FromStr;

use neptune_cash::api::export::BlockHeight;
use neptune_cash::api::export::Digest;
use neptune_cash::prelude::triton_vm::prelude::BFieldElement;
use neptune_cash::protocol::consensus::block::block_selector::BlockSelector;
use neptune_cash::protocol::consensus::block::block_selector::BlockSelectorParseError;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;

use super::height_or_digest::HeightOrDigest;

/// newtype for `BlockSelector` that provides ability to parse `height_or_digest/value`.
///
/// This is useful for HTML form(s) that allow user to enter either height or
/// digest into the same text input field.
///
/// In particular it is necessary to support javascript-free website with such
/// an html form.
#[derive(Debug, Clone)]
pub struct BlockSelectorExtended(BlockSelector);

impl std::fmt::Display for BlockSelectorExtended {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for BlockSelectorExtended {
    type Err = BlockSelectorParseError;

    // note: this parses BlockSelector, plus height_or_digest/<value>
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = match BlockSelector::from_str(s) {
            Ok(bs) => Ok(Self::from(bs)),
            Err(e) => {
                let parts: Vec<_> = s.split('/').collect();
                if parts.len() == 2 {
                    if parts[0] == "height_or_digest" {
                        Ok(Self::from(HeightOrDigest::from_str(parts[1])?))
                    } else if parts[0] == "digest" {
                        Ok(Self(BlockSelector::Digest(
                            Digest::try_from_hex(parts[1]).map_err(|tfhde| {
                                BlockSelectorParseError::InvalidSelector(tfhde.to_string())
                            })?,
                        )))
                    } else if parts[0] == "height" {
                        Ok(Self(BlockSelector::Height(BlockHeight::new(
                            BFieldElement::new(parts[1].parse().map_err(|e: ParseIntError| {
                                BlockSelectorParseError::InvalidSelector(e.to_string())
                            })?),
                        ))))
                    } else {
                        Err(e)
                    }
                } else {
                    Err(e)
                }
            }
        };
        res
    }
}

// note: axum uses serde Deserialize for Path elements.
impl<'de> Deserialize<'de> for BlockSelectorExtended {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(D::Error::custom)
    }
}

impl From<HeightOrDigest> for BlockSelectorExtended {
    fn from(hd: HeightOrDigest) -> Self {
        Self(hd.into())
    }
}

impl From<BlockSelector> for BlockSelectorExtended {
    fn from(v: BlockSelector) -> Self {
        Self(v)
    }
}

impl From<BlockSelectorExtended> for BlockSelector {
    fn from(v: BlockSelectorExtended) -> Self {
        v.0
    }
}
