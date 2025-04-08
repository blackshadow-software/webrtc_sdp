use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast::{self, Receiver};

pub type ClientBuffer = OnceLock<Mutex<broadcast::Sender<Vec<u8>>>>;
pub static CLIENT_BROADCAST_ENABLE: OnceLock<Mutex<bool>> = OnceLock::new();
pub static CLIENT_BUFFER: ClientBuffer = OnceLock::new();
pub const BUFFER: &str = "buffer";
// ? for client
pub fn set_client_boradcast_enable(enable: bool) {
    let v = CLIENT_BROADCAST_ENABLE.get_or_init(|| Mutex::new(enable));
    *v.lock().unwrap() = enable;
}

pub fn get_client_boradcast_enable() -> bool {
    *CLIENT_BROADCAST_ENABLE
        .get_or_init(|| Mutex::new(false))
        .lock()
        .unwrap()
}
pub fn init_client_buffer() -> Receiver<Vec<u8>> {
    let (tx, rx) = broadcast::channel::<Vec<u8>>(3);
    let _unused = CLIENT_BUFFER.get_or_init(|| Mutex::new(tx)).lock().unwrap();
    println!("Client buffer initialized");
    rx
}

pub fn add_bytes_in_client_buffer(bytes: Vec<u8>) {
    let (tx, _) = broadcast::channel::<Vec<u8>>(3);
    let set = CLIENT_BUFFER.get_or_init(|| Mutex::new(tx)).lock().unwrap();

    match set.send(bytes) {
        Ok(_) => println!("Bytes sent to broadcast"),
        Err(e) => {
            eprintln!("Error to send bytes to broadcast{:?}", e);
        }
    }
}

pub fn get_client_buffer_sender() -> broadcast::Sender<Vec<u8>> {
    let (tx, _) = broadcast::channel::<Vec<u8>>(3);
    let set = CLIENT_BUFFER.get_or_init(|| Mutex::new(tx));
    let set = set.lock().unwrap();
    set.clone()
}
