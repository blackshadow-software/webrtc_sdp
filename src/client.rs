use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use scrap::Display;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio_tungstenite::connect_async;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::interceptor::registry::Registry;
use webrtc::media::Sample;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

use crate::broad_cast::{
    add_bytes_in_client_buffer, get_client_boradcast_enable, init_client_buffer,
    set_client_boradcast_enable,
};
use crate::screen_capture::capture_screen;

const SIGNALING_SERVER: &str = "ws://127.0.0.1:8080"; // Modify if using WebSocket
pub const FPS_LIMIT: f64 = 30.0;

/// Start WebRTC screen-sharing session
pub async fn start_screen_share() -> Result<()> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let registry = Registry::new();
    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();
    println!("-----------------1");
    let config = RTCConfiguration::default();
    let peer_connection = Arc::new(api.new_peer_connection(config).await?);
    println!("-----------------2");

    let screen_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: "video/vp8".to_string(),
            clock_rate: 90000,
            ..Default::default()
        },
        "video".to_string(),
        "screen_share".to_string(),
    ));
    println!("-----------------3");

    peer_connection
        .add_track(Arc::clone(&screen_track) as Arc<dyn TrackLocal + Send + Sync>)
        .await?;
    println!("-----------------4");

    let sdp_offer = peer_connection.create_offer(None).await?;
    peer_connection
        .set_local_description(sdp_offer.clone())
        .await?;
    println!("-----------------5");

    let sdp_answer = send_sdp_offer(sdp_offer).await?;
    let answer: RTCSessionDescription = serde_json::from_str(&sdp_answer)?;
    println!("-----------------6");

    peer_connection.set_remote_description(answer).await?;
    println!("Connected to admin!");
    println!("-----------------7");

    start_screen_capture_loop(screen_track).await;
    println!("-----------------8");
    Ok(())
}

async fn send_sdp_offer(sdp_offer: RTCSessionDescription) -> Result<String> {
    let (mut ws_stream, _) = connect_async(SIGNALING_SERVER).await?;
    let sdp_json = serde_json::to_string(&sdp_offer)?;
    ws_stream
        .send(tokio_tungstenite::tungstenite::protocol::Message::Text(
            sdp_json.into(),
        ))
        .await?;

    while let Some(msg) = ws_stream.next().await {
        if let Ok(tokio_tungstenite::tungstenite::protocol::Message::Text(sdp_answer)) = msg {
            return Ok(sdp_answer.to_string());
        }
    }

    Err(anyhow::anyhow!("Failed to receive SDP Answer"))
}

async fn start_screen_capture_loop(track: Arc<TrackLocalStaticSample>) {
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
        Err(e) => {
            println!("Failed to find primary Display with : {:?}", e);
            return;
        }
    }
    loop {
        tokio::select! {                      Ok(buffer) = buffer_receiver.recv() => {
            let b= buffer;
            match track
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

/// Entry function to start the WebRTC screen share client
pub async fn run_client() -> Result<()> {
    match start_screen_share().await {
        Ok(_) => println!("Screen sharing started successfully."),
        Err(e) => eprintln!("Error: {}", e),
    }
    Ok(())
}
