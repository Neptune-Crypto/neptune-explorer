use neptune_cash::api::export::Announcement;
use neptune_cash::api::export::TransparentInput;
use neptune_cash::api::export::TransparentTransactionInfo;
use neptune_cash::prelude::triton_vm::prelude::BFieldCodec;
use neptune_cash::prelude::triton_vm::prelude::BFieldElement;

/// Magic first element identifying an announcement as a *lustration*: the
/// forced, on-chain revelation of a spent input whose AOCL leaf index is at or
/// below the network's lustration threshold.
///
/// Mirrors `LUSTRATION_FLAG` in neptune-core's `transaction_kernel`, which is
/// declared `pub(crate)` there and so cannot be imported here. The value is a
/// consensus-fixed constant; if it ever changes in neptune-core, this must be
/// updated to match.
const LUSTRATION_FLAG: BFieldElement = BFieldElement::new(51022176260);

#[derive(Debug, Clone)]
pub enum AnnouncementType {
    Unknown(Vec<BFieldElement>),
    TransparentTxInfo(TransparentTransactionInfo),
    /// A lustration announcement. The carried [`TransparentInput`] is the
    /// plaintext of the spent input that was lustrated (publicly revealed).
    Lustration(TransparentInput),
}

impl AnnouncementType {
    pub fn parse(announcement: Announcement) -> Self {
        // A lustration announcement's message is the lustration flag followed
        // by the BFieldCodec encoding of the revealed `TransparentInput` (see
        // `UnlockedUtxo::lustration` in neptune-core). This mirrors the
        // detection in neptune-core's own `verified_lustration_amount`. If the
        // payload fails to decode, fall through to the generic handling below.
        if announcement.message.first() == Some(&LUSTRATION_FLAG) {
            if let Ok(transparent_input) = TransparentInput::decode(&announcement.message[1..]) {
                return Self::Lustration(*transparent_input);
            }
        }

        if let Ok(transparent_transaction_info) =
            TransparentTransactionInfo::try_from_announcement(&announcement)
        {
            Self::TransparentTxInfo(transparent_transaction_info)
        } else {
            Self::Unknown(announcement.message)
        }
    }

    pub fn name(&self) -> String {
        match self {
            AnnouncementType::Unknown(_) => "unknown",
            AnnouncementType::TransparentTxInfo(_) => "transparent transaction info",
            AnnouncementType::Lustration(_) => "lustration",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use neptune_cash::api::export::Announcement;
    use neptune_cash::api::export::TransparentInput;
    use neptune_cash::api::export::Utxo;
    use neptune_cash::prelude::tasm_lib::prelude::Digest;
    use neptune_cash::prelude::triton_vm::prelude::BFieldCodec;
    use neptune_cash::prelude::triton_vm::prelude::BFieldElement;

    use super::AnnouncementType;
    use super::LUSTRATION_FLAG;

    /// Build a lustration announcement's message: the flag followed by the
    /// BFieldCodec encoding of a `TransparentInput`, exactly as neptune-core's
    /// `UnlockedUtxo::lustration()` constructs it.
    fn lustration_message(transparent_input: &TransparentInput) -> Vec<BFieldElement> {
        let mut message = vec![LUSTRATION_FLAG];
        message.extend(transparent_input.encode());
        message
    }

    fn sample_transparent_input() -> TransparentInput {
        TransparentInput {
            utxo: Utxo::new(Digest::default(), vec![]),
            aocl_leaf_index: 42,
            sender_randomness: Digest::default(),
            receiver_preimage: Digest::default(),
        }
    }

    #[test]
    fn parse_decodes_lustration_announcement() {
        let transparent_input = sample_transparent_input();
        let announcement = Announcement::new(lustration_message(&transparent_input));

        // `TransparentInput` does not derive `PartialEq` outside of
        // neptune-core's own test build, so verify the round-trip by comparing
        // the re-encoded sequence and a known field.
        match AnnouncementType::parse(announcement) {
            AnnouncementType::Lustration(decoded) => {
                assert_eq!(transparent_input.encode(), decoded.encode());
                assert_eq!(42, decoded.aocl_leaf_index);
            }
            other => panic!("expected Lustration, got {other:?}"),
        }
    }

    #[test]
    fn lustration_name_is_lustration() {
        let announcement = Announcement::new(lustration_message(&sample_transparent_input()));
        let parsed = AnnouncementType::parse(announcement);
        assert_eq!("lustration", parsed.name());
    }

    #[test]
    fn parse_falls_back_to_unknown_when_payload_does_not_decode() {
        // Flag is present but the remainder is not a valid `TransparentInput`
        // encoding. Must not panic and must not be classified as Lustration.
        let announcement = Announcement::new(vec![LUSTRATION_FLAG, BFieldElement::new(7)]);
        match AnnouncementType::parse(announcement) {
            AnnouncementType::Unknown(_) => {}
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn parse_falls_back_to_unknown_for_flag_only_announcement() {
        // Flag with no payload at all (empty slice after the flag).
        let announcement = Announcement::new(vec![LUSTRATION_FLAG]);
        match AnnouncementType::parse(announcement) {
            AnnouncementType::Unknown(_) => {}
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn parse_leaves_non_lustration_announcements_unchanged() {
        // An announcement whose first element is not the lustration flag must
        // never be classified as a lustration, regardless of its payload.
        let announcement = Announcement::new(vec![
            BFieldElement::new(7), // not the lustration flag
            BFieldElement::new(13),
        ]);
        let parsed = AnnouncementType::parse(announcement);
        assert!(
            !matches!(parsed, AnnouncementType::Lustration(_)),
            "non-lustration announcement must not parse as Lustration, got {parsed:?}"
        );
    }
}
