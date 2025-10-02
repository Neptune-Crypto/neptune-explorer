use std::fmt::Display;
use std::str::FromStr;

use neptune_cash::api::export::BlockHeight;
use neptune_cash::prelude::tasm_lib::prelude::Digest;
use neptune_cash::protocol::consensus::block::block_selector::BlockSelector;
use serde::de::Error;
use serde::Deserialize;
use serde::Deserializer;

/// newtype for `BlockSelector` that provides ability to parse `height_or_digest/value`.
///
/// This is useful for HTML form(s) that allow user to enter either height or
/// digest into the same text input field.
///
/// In particular it is necessary to support javascript-free website with such
/// an html form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnouncementSelector {
    pub block_selector: BlockSelector,
    pub index: usize,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AnnouncementSelectorParseError {
    #[error("too many or too few parts in announcement path")]
    NumberOfParts,
    #[error("error parsing index for announcement in tip: {0}")]
    TipIndex(std::num::ParseIntError),
    #[error("error parsing index for announcement in genesis block: {0}")]
    GenesisIndex(std::num::ParseIntError),
    #[error("error parsing block height: {0}")]
    BlockHeight(std::num::ParseIntError),
    #[error("error parsing index for announcement in block {0}: {1}")]
    HeightIndex(BlockHeight, std::num::ParseIntError),
    #[error("error parsing digest: {0}")]
    BlockDigest(neptune_cash::prelude::twenty_first::error::TryFromHexDigestError),
    #[error("error parsing index for announcement in block {0}: {1}")]
    DigestIndex(Digest, std::num::ParseIntError),
    #[error("error parsing block-height-or-digest: {0} / {1}")]
    HeightNorDigest(
        std::num::ParseIntError,
        neptune_cash::prelude::twenty_first::error::TryFromHexDigestError,
    ),
    #[error("error parsing index for block-height-or-digest {0}: {1}")]
    HeightOrDigestIndex(BlockSelector, std::num::ParseIntError),
    #[error("invalid keyword {0} or {1}")]
    InvalidKeyword(String, String),
    #[error("invalid prefix: {0}")]
    InvalidPrefix(String),
}

impl FromStr for AnnouncementSelector {
    type Err = AnnouncementSelectorParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = input.split('/').collect();

        let (block_selector, index) = match parts.as_slice() {
            ["tip", index] => {
                let index = index.parse::<u64>().map_err(Self::Err::TipIndex)?;
                (BlockSelector::Tip, index)
            }
            ["genesis", index] => index
                .parse::<u64>()
                .map(|i| (BlockSelector::Genesis, i))
                .map_err(Self::Err::GenesisIndex)?,
            ["height", number, index] => {
                let height_as_u64 = number.parse::<u64>().map_err(Self::Err::BlockHeight)?;
                let block_height = BlockHeight::from(height_as_u64);
                (
                    BlockSelector::Height(block_height),
                    index
                        .parse()
                        .map_err(|e| Self::Err::HeightIndex(block_height, e))?,
                )
            }
            ["digest", hash, index] => {
                let digest = Digest::try_from_hex(hash).map_err(Self::Err::BlockDigest)?;
                let index = index
                    .parse::<u64>()
                    .map_err(|e| Self::Err::DigestIndex(digest, e))?;
                (BlockSelector::Digest(digest), index)
            }
            ["height_or_digest", hod, "index", index] => {
                let parsed_height = hod.parse::<u64>();
                let parsed_digest = Digest::try_from_hex(hod);

                let block_selector = match (parsed_height, parsed_digest) {
                    (Ok(_), Ok(digest)) => {
                        // unreachable? Not in theory ...
                        BlockSelector::Digest(digest)
                    }
                    (Ok(h), Err(_)) => BlockSelector::Height(BlockHeight::from(h)),
                    (Err(_), Ok(digest)) => BlockSelector::Digest(digest),
                    (Err(pie), Err(hde)) => {
                        return Err(Self::Err::HeightNorDigest(pie, hde));
                    }
                };

                let index = index
                    .parse::<u64>()
                    .map_err(|e| Self::Err::HeightOrDigestIndex(block_selector, e))?;
                (block_selector, index)
            }
            [prefix, _, keyword, _] => {
                return Err(Self::Err::InvalidKeyword(
                    prefix.to_string(),
                    keyword.to_string(),
                ))
            }
            [prefix, _, _] | [prefix, _] => {
                return Err(Self::Err::InvalidPrefix(prefix.to_string()))
            }
            &[] | &[_] | &[_, _, _, _, _, ..] => return Err(Self::Err::NumberOfParts),
        };

        Ok(AnnouncementSelector {
            block_selector,
            index: index as usize,
        })
    }
}

impl Display for AnnouncementSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.block_selector {
            BlockSelector::Digest(digest) => write!(f, "digest/{digest:x}/{}", self.index),
            BlockSelector::Height(block_height) => {
                write!(f, "height/{}/{}", block_height, self.index)
            }
            BlockSelector::Genesis => write!(f, "genesis/{}", self.index),
            BlockSelector::Tip => write!(f, "tip/{}", self.index),
        }
    }
}

// note: axum uses serde Deserialize for Path elements.
impl<'de> Deserialize<'de> for AnnouncementSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use arbitrary::Arbitrary;
    use arbitrary::Unstructured;
    use proptest::prop_assert;
    use proptest::prop_assert_eq;
    use proptest::string::string_regex;
    use proptest_arbitrary_interop::arb;
    use test_strategy::proptest;

    use super::*;

    impl<'a> Arbitrary<'a> for AnnouncementSelector {
        fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
            // Pick one of the variants randomly
            let variant = u.int_in_range(0..=3)?;

            let selector = match variant {
                0 => {
                    // Digest selector
                    let digest = Digest::arbitrary(u)?;
                    let index = u64::arbitrary(u)? as usize;
                    AnnouncementSelector {
                        block_selector: BlockSelector::Digest(digest),
                        index,
                    }
                }
                1 => {
                    // Height selector
                    let height_u64 = u64::arbitrary(u)?;
                    let bh = BlockHeight::from(height_u64);
                    let index = u64::arbitrary(u)? as usize;
                    AnnouncementSelector {
                        block_selector: BlockSelector::Height(bh),
                        index,
                    }
                }
                2 => {
                    // Genesis selector
                    let index = u64::arbitrary(u)? as usize;
                    AnnouncementSelector {
                        block_selector: BlockSelector::Genesis,
                        index,
                    }
                }
                3 => {
                    // Tip selector
                    let index = u64::arbitrary(u)? as usize;
                    AnnouncementSelector {
                        block_selector: BlockSelector::Tip,
                        index,
                    }
                }
                _ => unreachable!(),
            };

            Ok(selector)
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            // Not very precise, but enough for fuzzing/proptesting
            (1, None)
        }
    }

    #[proptest]
    fn display_roundtrip(#[strategy(arb())] announcement_selector: AnnouncementSelector) {
        let as_string = announcement_selector.to_string();
        let parsed = AnnouncementSelector::from_str(&as_string).unwrap();
        prop_assert_eq!(announcement_selector, parsed.clone());

        let as_string_again = parsed.to_string();
        prop_assert_eq!(as_string, as_string_again);
    }

    #[proptest]
    fn parse_height_or_digest_digest(
        #[strategy(arb())] digest: Digest,
        #[strategy(0usize..20)] index: usize,
    ) {
        let str = format!("height_or_digest/{digest:x}/index/{index}");
        AnnouncementSelector::from_str(&str).unwrap(); // no crash
    }

    #[proptest]
    fn parse_height_or_digest_height(
        #[strategy(arb())] block_height: u64,
        #[strategy(0usize..20)] index: usize,
    ) {
        let str = format!("height_or_digest/{block_height}/index/{index}");
        AnnouncementSelector::from_str(&str).unwrap(); // no crash
    }

    #[proptest]
    fn parse_invalid_number_of_parts(s: String) {
        // Strings with fewer than 2 parts OR more than 3 parts should fail
        let parts: Vec<&str> = s.split('/').collect();
        if !(2..=4).contains(&parts.len()) {
            let res = AnnouncementSelector::from_str(&s);
            prop_assert!(matches!(
                res,
                Err(AnnouncementSelectorParseError::NumberOfParts)
            ));
        }
    }

    #[proptest]
    fn parse_invalid_tip_index(#[strategy(string_regex("tip/[a-z]+").unwrap())] s: String) {
        let res = AnnouncementSelector::from_str(&s);
        prop_assert!(matches!(
            res,
            Err(AnnouncementSelectorParseError::TipIndex(_))
        ));
    }

    #[proptest]
    fn parse_invalid_genesis_index(#[strategy(string_regex("genesis/[a-z]+").unwrap())] s: String) {
        let res = AnnouncementSelector::from_str(&s);
        prop_assert!(matches!(
            res,
            Err(AnnouncementSelectorParseError::GenesisIndex(_))
        ));
    }

    #[proptest]
    fn parse_invalid_height_number(
        #[strategy(string_regex("height/[a-z]+/0").unwrap())] s: String,
    ) {
        let res = AnnouncementSelector::from_str(&s);
        prop_assert!(matches!(
            res,
            Err(AnnouncementSelectorParseError::BlockHeight(_))
        ));
    }

    #[proptest]
    fn parse_invalid_height_index(
        #[strategy(string_regex("height/42/[a-z]+").unwrap())] s: String,
    ) {
        let res = AnnouncementSelector::from_str(&s);
        if let Err(AnnouncementSelectorParseError::HeightIndex(_, _)) = res {
            // OK
        } else {
            panic!("Expected HeightIndex error, got {res:?}");
        }
    }

    #[proptest]
    fn parse_invalid_digest_hex(
        #[strategy(string_regex("digest/z[0-9a-f]{79}/0").unwrap())] s: String,
    ) {
        let res = AnnouncementSelector::from_str(&s);
        prop_assert!(matches!(
            res,
            Err(AnnouncementSelectorParseError::BlockDigest(_))
        ));
    }

    #[proptest]
    fn parse_invalid_digest_length_too_short(
        #[strategy(string_regex("digest/[0-9a-f]{0,79}/0").unwrap())] s: String,
    ) {
        let res = AnnouncementSelector::from_str(&s);
        prop_assert!(matches!(
            res,
            Err(AnnouncementSelectorParseError::BlockDigest(_))
        ));
    }

    #[proptest]
    fn parse_invalid_digest_length_too_long(
        #[strategy(string_regex("digest/[0-9a-f]{81,200}/0").unwrap())] s: String,
    ) {
        let res = AnnouncementSelector::from_str(&s);
        prop_assert!(matches!(
            res,
            Err(AnnouncementSelectorParseError::BlockDigest(_))
        ));
    }

    #[proptest]
    fn parse_invalid_digest_index(#[strategy(arb())] digest: Digest) {
        let s = format!("digest/{digest:x}/notanumber");
        let res = AnnouncementSelector::from_str(&s);
        prop_assert!(matches!(
            res,
            Err(AnnouncementSelectorParseError::DigestIndex(_, _))
        ));
    }
}
