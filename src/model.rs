use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;

use crate::CLIENT_SDP_OFFER;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct SdpOfferAnswer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
}

impl SdpOfferAnswer {
    pub fn new(offer: Option<String>, answer: Option<String>, client_id: Option<String>) -> Self {
        SdpOfferAnswer {
            flag: Some(CLIENT_SDP_OFFER.to_string()),
            offer,
            answer,
            client_id,
        }
    }

    pub fn to_ws(&self) -> Message {
        Message::text(serde_json::to_string(&self).unwrap())
    }
}
