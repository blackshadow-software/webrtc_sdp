pub mod broad_cast;
pub mod client;
pub mod model;
pub mod screen_capture;
pub mod sdp;
pub const CLIENT_SDP_OFFER: &str = "client_sdp_offer";

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
