#[derive(Clone)]
pub struct CameraInfo {
    pub index: usize,
    pub name: String,
    pub device_path: String,
}

pub struct StreamConfig {
    pub camera_index: usize,
    pub ip: String,
    pub port: String,
    pub fec_port: String,
}
