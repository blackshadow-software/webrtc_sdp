use anyhow::{bail, Result};
use scrap::Display;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::{runtime::Runtime, sync::oneshot};
use tokio_tungstenite::tungstenite::Message;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;
use webrtc::{
    api::interceptor_registry::register_default_interceptors, interceptor::registry::Registry,
};
use webrtc::{api::media_engine::MediaEngine, media::Sample};
use webrtc::{api::APIBuilder, ice_transport::ice_server::RTCIceServer};
use webrtc::{
    ice_transport::ice_candidate::RTCIceCandidate, peer_connection::configuration::RTCConfiguration,
};
use webrtc::{
    ice_transport::ice_candidate::RTCIceCandidateInit,
    peer_connection::sdp::session_description::RTCSessionDescription,
};

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

pub async fn init_sdp() -> Result<()> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)?;
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };
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

    let rtpc = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build()
        .new_peer_connection(config)
        .await?;
    rtpc.add_track(Arc::clone(&screen_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;
    RTC_CONFIG.get_or_init(|| Mutex::new(rtpc));
    Ok(())
}

pub async fn my_ice_candidate() -> Result<String> {
    let (tx, rx) = oneshot::channel::<String>();
    let tx_arc = Arc::new(Mutex::new(Some(tx)));

    let rtpc = RTC_CONFIG.get().unwrap().lock().unwrap();
    rtpc.on_ice_candidate(Box::new({
        let tx_arc = Arc::clone(&tx_arc);
        move |candidate: Option<RTCIceCandidate>| {
            if let Some(c) = candidate {
                if let Ok(ice) = c.to_json() {
                    if let Some(tx) = tx_arc.lock().unwrap().take() {
                        let _ = tx.send(ice.candidate);
                    }
                }
            }
            Box::pin(async move {})
        }
    }));

    let candidate_string = rx.await?;
    Ok(candidate_string)
}

pub async fn create_sdp_offer() -> Result<SdpOfferAnswer> {
    let rtpc = RTC_CONFIG.get().unwrap().lock().unwrap();
    let sdp_offer = rtpc.create_offer(None).await?;
    rtpc.set_local_description(sdp_offer.clone()).await?;
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
    let rtpc = RTC_CONFIG.get().unwrap().lock().unwrap();
    rtpc.set_remote_description(offer).await?;
    let sdp_answer = rtpc.create_answer(None).await?;
    rtpc.set_local_description(sdp_answer.clone()).await?;

    let offer = SdpOfferAnswer::new(
        None,
        Some(sdp_answer.to_json()),
        Some(client_id.to_string()),
    );

    Ok(offer.to_ws())
}

pub async fn set_ice_candidate(ice: String) -> Result<()> {
    let candidate: RTCIceCandidateInit = RTCIceCandidateInit {
        candidate: ice,
        ..Default::default()
    };
    let rtpc = RTC_CONFIG.get().unwrap().lock().unwrap();
    rtpc.add_ice_candidate(candidate).await?;
    Ok(())
}

pub fn start_screen_capture_loop() -> Result<()> {
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
    std::thread::spawn(move || {
        Runtime::new().unwrap().block_on(async {
            let mut buffer_receiver = init_client_buffer();
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
        });
    });
    Ok(())
}
