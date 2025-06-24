use crate::gstreamer::GStreamerPipeline;
use crate::stats_collector::{StatsCollector, StreamStats};
use crate::types::{CameraState, CameraConfig};
use std::sync::Arc;

pub struct CameraBackend {
    cameras: Vec<CameraState>,
    pipelines: Vec<Option<GStreamerPipeline>>,
    stats_collectors: Vec<Option<Arc<std::sync::Mutex<StatsCollector>>>>,
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
            stats_collectors: vec![None, None, None, None],
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
        
        let rtp_port = camera.config.rtp_port.parse::<u16>().unwrap_or(5000);
        let fec_port = camera.config.fec_port.parse::<u16>().unwrap_or(5002);
        
        let mut stats_collector = StatsCollector::new(camera_index, rtp_port, fec_port);
        if let Err(e) = stats_collector.start() {
            println!("Warning: Failed to start stats collector for camera {}: {}", camera_index + 1, e);
        }
        
        let stats_collector_arc = Arc::new(std::sync::Mutex::new(stats_collector));
        self.stats_collectors[camera_index] = Some(Arc::clone(&stats_collector_arc));
        
        let mut pipeline = GStreamerPipeline::new(
            camera_index,
            camera.config.clone(),
            Arc::clone(&camera.receiving),
            Some(stats_collector_arc),
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

        if let Some(stats_collector_arc) = self.stats_collectors[camera_index].take() {
            if let Ok(mut stats_collector) = stats_collector_arc.lock() {
                stats_collector.stop();
            }
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
        if camera_index < self.stats_collectors.len() {
            if let Some(ref stats_collector_arc) = self.stats_collectors[camera_index] {
                if let Ok(stats_collector) = stats_collector_arc.lock() {
                    Some(stats_collector.get_stats())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn parse_debug_line(&mut self, camera_index: usize, line: &str) {
        if camera_index < self.stats_collectors.len() {
            if let Some(ref mut stats_collector) = self.stats_collectors[camera_index] {
                if let Ok(mut collector) = stats_collector.lock() {
                    collector.parse_debug_line(line);
                }
            }
        }
    }

    pub fn get_camera_config(&self, camera_index: usize) -> Option<&CameraConfig> {
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
