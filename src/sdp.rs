use anyhow::Result;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

use crate::model::{SdpImpl, SdpOfferAnswer};

pub async fn create_sdp_offer() -> Result<SdpOfferAnswer> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let registry = Registry::new();
    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();
    let config = RTCConfiguration::default();
    let peer_connection = Arc::new(api.new_peer_connection(config).await?);

    let screen_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: "video/vp8".to_string(),
            clock_rate: 90000,
            ..Default::default()
        },
        "video".to_string(),
        "screen_share".to_string(),
    ));

    peer_connection
        .add_track(Arc::clone(&screen_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;

    let sdp_offer = peer_connection.create_offer(None).await?;
    let offer = SdpOfferAnswer::new(Some(sdp_offer.to_json()), None, None);

    Ok(offer)
}

pub async fn create_sdp_answer(sdp_offer: String, client_id: &str) -> Result<Message> {
    let offer: RTCSessionDescription = serde_json::from_str(&sdp_offer)?;

    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let registry = Registry::new();
    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration::default();
    let peer_connection = Arc::new(api.new_peer_connection(config).await?);

    peer_connection.set_remote_description(offer).await?;
    let sdp_answer = peer_connection.create_answer(None).await?;
    let offer = SdpOfferAnswer::new(
        None,
        Some(sdp_answer.to_json()),
        Some(client_id.to_string()),
    );

    Ok(offer.to_ws())
}
