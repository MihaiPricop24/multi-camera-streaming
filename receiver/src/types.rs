use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CameraConfig {
    pub sender_ip: String,
    pub rtp_port: String,
    pub fec_port: String,
}

pub struct CameraState {
    pub config: CameraConfig,
    pub receiving: Arc<Mutex<bool>>,
}

impl CameraState {
    pub fn new(sender_ip: &str, rtp_port: &str, fec_port: &str) -> Self {
        Self {
            config: CameraConfig {
                sender_ip: sender_ip.to_string(),
                rtp_port: rtp_port.to_string(),
                fec_port: fec_port.to_string(),
            },
            receiving: Arc::new(Mutex::new(false)),
        }
    }
}