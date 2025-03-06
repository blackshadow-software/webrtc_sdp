use anyhow::{bail, Result};
use scrap::Display;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio_tungstenite::tungstenite::Message;
use webrtc::api::APIBuilder;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;
use webrtc::{api::media_engine::MediaEngine, media::Sample};

use crate::{
    broad_cast::{
        add_bytes_in_client_buffer, get_client_boradcast_enable, init_client_buffer,
        set_client_boradcast_enable,
    },
    client::FPS_LIMIT,
    model::{SdpImpl, SdpOfferAnswer},
    screen_capture::capture_screen,
    RTC_CONFIG, RTC_TRACK,
};

pub async fn create_sdp_offer() -> Result<SdpOfferAnswer> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let registry = Registry::new();
    let config = RTCConfiguration::default();
    let screen_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: "video/vp8".to_string(),
            clock_rate: 90000,
            ..Default::default()
        },
        "video".to_string(),
        "screen_share".to_string(),
    ));
    RTC_TRACK.get_or_init(|| screen_track.clone());

    let peer_connection = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build()
        .new_peer_connection(config)
        .await?;

    peer_connection
        .add_track(Arc::clone(&screen_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;

    let sdp_offer = peer_connection.create_offer(None).await?;
    peer_connection
        .set_local_description(sdp_offer.clone())
        .await?;
    RTC_CONFIG.get_or_init(|| Mutex::new(peer_connection));
    let offer = SdpOfferAnswer::new(Some(sdp_offer.to_json()), None, None);

    Ok(offer)
}

pub async fn set_remote_answer_sdp(answer: &SdpOfferAnswer) -> Result<()> {
    let answer: RTCSessionDescription = serde_json::from_str(&answer.answer.clone().unwrap())?;

    let peer_conn = RTC_CONFIG.get().unwrap().lock().unwrap();
    peer_conn.set_remote_description(answer).await?;

    Ok(())
}

pub async fn create_sdp_answer(sdp_offer: String, client_id: &str) -> Result<Message> {
    println!("Received SDP offer: {:?}", sdp_offer);
    let offer: RTCSessionDescription = serde_json::from_str(&sdp_offer)?;

    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let registry = Registry::new();
    let config = RTCConfiguration::default();
    let peer_connection = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build()
        .new_peer_connection(config)
        .await?;

    peer_connection.set_remote_description(offer).await?;
    let sdp_answer = peer_connection.create_answer(None).await?;
    peer_connection
        .set_local_description(sdp_answer.clone())
        .await?;
    RTC_CONFIG.get_or_init(|| Mutex::new(peer_connection));

    let offer = SdpOfferAnswer::new(
        None,
        Some(sdp_answer.to_json()),
        Some(client_id.to_string()),
    );

    Ok(offer.to_ws())
}

pub async fn start_screen_capture_loop() -> Result<()> {
    let mut buffer_receiver = init_client_buffer();
    match Display::primary() {
        Ok(_) => {
            set_client_boradcast_enable(true);

            thread::spawn(move || {
                let frame_time = Duration::from_secs_f64(1.0 / FPS_LIMIT as f64);
                loop {
                    let start_time = Instant::now();

                    if get_client_boradcast_enable() == false {
                        break;
                    }

                    match capture_screen() {
                        Ok(screen_data) => {
                            if screen_data.is_empty() {
                                continue;
                            }
                            add_bytes_in_client_buffer(screen_data);
                            let elapsed = start_time.elapsed();
                            if elapsed < frame_time {
                                thread::sleep(frame_time - elapsed);
                            }
                        }
                        Err(e) => eprintln!("Failed to capture screen: {:?}", e),
                    }
                }
            });
        }
        Err(e) => bail!("Failed to find primary Display with : {:?}", e),
    }
    loop {
        tokio::select! {
          Ok(buffer) = buffer_receiver.recv() => {
            let b= buffer;
            match RTC_TRACK.get().unwrap()
            .write_sample(&Sample {
                data: b.into(),
                duration: Duration::from_millis(33),
                ..Default::default()
            })
            .await
            {
                Ok(_) => println!("Sent frame to WebRTC track"  ),
                Err(e) => eprintln!("Error sending fram {}",   e),
            }
        }}
    }
}
