use std::sync::{Arc, Mutex};

#[derive(Clone, Default, Debug)]
pub struct StreamStats {
    pub packets_received: u32,
    pub packets_lost: u32,
    pub packets_repaired: u32,
    pub repair_rate: f32,
    pub bitrate: f32,
    pub latency: f32,
}

#[derive(Clone)]
pub struct CameraConfig {
    pub sender_ip: String,
    pub rtp_port: String,
    pub fec_port: String,
}

pub struct CameraState {
    pub config: CameraConfig,
    pub receiving: Arc<Mutex<bool>>,
    pub stats: Arc<Mutex<StreamStats>>,
    pub gst_pid: Arc<Mutex<Option<u32>>>,
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
            stats: Arc::new(Mutex::new(StreamStats::default())),
            gst_pid: Arc::new(Mutex::new(None)),
        }
    }
}
