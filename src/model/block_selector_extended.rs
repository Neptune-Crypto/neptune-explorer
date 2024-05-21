use super::height_or_digest::HeightOrDigest;
use neptune_core::models::blockchain::block::block_selector::BlockSelector;
use neptune_core::models::blockchain::block::block_selector::BlockSelectorParseError;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;
use std::str::FromStr;

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
        match BlockSelector::from_str(s) {
            Ok(bs) => Ok(Self::from(bs)),
            Err(e) => {
                let parts: Vec<_> = s.split('/').collect();
                if parts.len() == 2 && parts[0] == "height_or_digest" {
                    Ok(Self::from(HeightOrDigest::from_str(parts[1])?))
                } else {
                    Err(e)
                }
            }
        }
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
