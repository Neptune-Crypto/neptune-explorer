use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use neptune_core::models::blockchain::block::block_height::BlockHeight;
use neptune_core::models::blockchain::block::block_selector::BlockSelector;
use neptune_core::prelude::tasm_lib::Digest;
use neptune_core::prelude::twenty_first::error::TryFromHexDigestError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PathBlockSelector {
    #[serde(rename = "genesis")]
    Genesis,
    #[serde(rename = "tip")]
    Tip,
    #[serde(rename = "digest")]
    Digest,
    #[serde(rename = "height")]
    Height,
    #[serde(rename = "height_or_digest")]
    HeightOrDigest,
}

#[derive(thiserror::Error, Debug)]
pub enum PathBlockSelectorError {
    #[error("Genesis does not accept an argument")]
    GenesisNoArg,

    #[error("Tip does not accept an argument")]
    TipNoArg,

    #[error("Digest could not be parsed")]
    DigestNotParsed(#[from] TryFromHexDigestError),

    #[error("Height could not be parsed")]
    HeightNotParsed(#[from] std::num::ParseIntError),
}
impl PathBlockSelectorError {
    fn as_response_tuple(&self) -> (StatusCode, String) {
        (StatusCode::NOT_FOUND, self.to_string())
    }
}
impl IntoResponse for PathBlockSelectorError {
    fn into_response(self) -> Response {
        self.as_response_tuple().into_response()
    }
}
impl From<PathBlockSelectorError> for Response {
    fn from(e: PathBlockSelectorError) -> Response {
        e.as_response_tuple().into_response()
    }
}
impl PathBlockSelector {
    pub fn as_block_selector(&self, value: &str) -> Result<BlockSelector, PathBlockSelectorError> {
        match self {
            PathBlockSelector::Genesis if !value.is_empty() => {
                Err(PathBlockSelectorError::GenesisNoArg)
            }
            PathBlockSelector::Genesis => Ok(BlockSelector::Genesis),
            PathBlockSelector::Tip if !value.is_empty() => Err(PathBlockSelectorError::TipNoArg),
            PathBlockSelector::Tip => Ok(BlockSelector::Tip),
            PathBlockSelector::Digest => Ok(BlockSelector::Digest(Digest::try_from_hex(value)?)),
            PathBlockSelector::Height => Ok(BlockSelector::Height(BlockHeight::from(
                u64::from_str(value)?,
            ))),
            PathBlockSelector::HeightOrDigest => Ok(match u64::from_str(value) {
                Ok(height) => BlockSelector::Height(BlockHeight::from(height)),
                Err(_) => BlockSelector::Digest(Digest::try_from_hex(value)?),
            }),
        }
    }
}
