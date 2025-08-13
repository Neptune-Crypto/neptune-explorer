use neptune_cash::api::export::Announcement;
use neptune_cash::api::export::TransparentTransactionInfo;
use neptune_cash::prelude::triton_vm::prelude::BFieldElement;

#[derive(Debug, Clone)]
pub enum AnnouncementType {
    Unknown(Vec<BFieldElement>),
    TransparentTxInfo(TransparentTransactionInfo),
}

impl AnnouncementType {
    pub fn parse(announcement: Announcement) -> Self {
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
        }
        .to_string()
    }
}
