use crate::gstreamer::GStreamerPipeline;
use crate::types::{CameraState, StreamStats};
use std::sync::Arc;

pub struct CameraBackend {
    cameras: Vec<CameraState>,
    pipelines: Vec<Option<GStreamerPipeline>>,
}

impl CameraBackend {
    pub fn new() -> Self {
        let cameras = vec![
            CameraState::new("192.168.0.105", "5000", "5002"),
            CameraState::new("192.168.0.105", "5004", "5006"),
            CameraState::new("192.168.0.105", "5008", "5010"),
            CameraState::new("192.168.0.105", "5012", "5014"),
        ];

        Self {
            cameras,
            pipelines: vec![None, None, None, None],
        }
    }

    pub fn update_camera_config(
        &mut self,
        camera_index: usize,
        sender_ip: &str,
        rtp_port: &str,
        fec_port: &str,
    ) {
        if camera_index < self.cameras.len() {
            self.cameras[camera_index].config.sender_ip = sender_ip.to_string();
            self.cameras[camera_index].config.rtp_port = rtp_port.to_string();
            self.cameras[camera_index].config.fec_port = fec_port.to_string();
        }
    }

    pub fn start_camera(&mut self, camera_index: usize) -> Result<(), String> {
        if camera_index >= self.cameras.len() {
            return Err("Invalid camera index".to_string());
        }

        if self.is_camera_running(camera_index) {
            return Err("Camera already running".to_string());
        }

        let camera = &self.cameras[camera_index];
        let mut pipeline = GStreamerPipeline::new(
            camera_index,
            camera.config.clone(),
            Arc::clone(&camera.receiving),
            Arc::clone(&camera.stats),
            Arc::clone(&camera.gst_pid),
        );

        pipeline.start();
        self.pipelines[camera_index] = Some(pipeline);

        Ok(())
    }

    pub fn stop_camera(&mut self, camera_index: usize) -> Result<(), String> {
        if camera_index >= self.cameras.len() {
            return Err("Invalid camera index".to_string());
        }

        if let Some(mut pipeline) = self.pipelines[camera_index].take() {
            pipeline.stop();
        }

        Ok(())
    }

    pub fn toggle_camera(&mut self, camera_index: usize) -> Result<(), String> {
        if self.is_camera_running(camera_index) {
            self.stop_camera(camera_index)
        } else {
            self.start_camera(camera_index)
        }
    }

    pub fn is_camera_running(&self, camera_index: usize) -> bool {
        if camera_index < self.cameras.len() {
            *self.cameras[camera_index].receiving.lock().unwrap()
        } else {
            false
        }
    }

    pub fn get_camera_stats(&self, camera_index: usize) -> Option<StreamStats> {
        if camera_index < self.cameras.len() {
            Some(self.cameras[camera_index].stats.lock().unwrap().clone())
        } else {
            None
        }
    }

    pub fn get_camera_config(&self, camera_index: usize) -> Option<&crate::types::CameraConfig> {
        if camera_index < self.cameras.len() {
            Some(&self.cameras[camera_index].config)
        } else {
            None
        }
    }

    pub fn shutdown(&mut self) {
        for i in 0..self.cameras.len() {
            let _ = self.stop_camera(i);
        }
    }
}
