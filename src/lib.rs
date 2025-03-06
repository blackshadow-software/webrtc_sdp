use std::sync::{Arc, Mutex, OnceLock};

use webrtc::{
    peer_connection::RTCPeerConnection,
    track::track_local::track_local_static_sample::TrackLocalStaticSample,
};

pub mod broad_cast;
pub mod client;
pub mod model;
pub mod screen_capture;
pub mod sdp;
pub const CLIENT_SDP_OFFER: &str = "client_sdp_offer";
pub static RTC_CONFIG: OnceLock<Mutex<RTCPeerConnection>> = OnceLock::new();
pub static RTC_TRACK: OnceLock<Arc<TrackLocalStaticSample>> = OnceLock::new();

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
