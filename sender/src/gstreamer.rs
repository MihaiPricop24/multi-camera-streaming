use crate::types::StreamConfig;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct GStreamerManager {
    pub streaming: Vec<Arc<Mutex<bool>>>,
    pub stream_threads: Vec<Option<thread::JoinHandle<()>>>,
    pub gst_pids: Vec<Arc<Mutex<Option<u32>>>>,
}

impl GStreamerManager {
    pub fn new() -> Self {
        Self {
            streaming: Vec::new(),
            stream_threads: Vec::new(),
            gst_pids: Vec::new(),
        }
    }

    pub fn initialize_streams(&mut self, count: usize) {
        for _ in 0..count {
            self.streaming.push(Arc::new(Mutex::new(false)));
            self.stream_threads.push(None);
            self.gst_pids.push(Arc::new(Mutex::new(None)));
        }
    }

    pub fn start_pipeline(&mut self, control_index: usize, config: StreamConfig) {
        *self.streaming[control_index].lock().unwrap() = true;

        let streaming = Arc::clone(&self.streaming[control_index]);
        let gst_pid = Arc::clone(&self.gst_pids[control_index]);

        self.stream_threads[control_index] = Some(thread::spawn(move || {
            Self::run_pipeline_with_fallback(control_index, config, streaming, gst_pid);
        }));
    }

    fn run_pipeline_with_fallback(
        control_index: usize,
        config: StreamConfig,
        streaming: Arc<Mutex<bool>>,
        gst_pid: Arc<Mutex<Option<u32>>>,
    ) {
        // Try different pipeline configurations for virtual cameras
        let pipeline_configs = vec![
            // Configuration 1: Standard pipeline
            Self::build_standard_pipeline(&config, control_index),
            // Configuration 2: With explicit caps for virtual cameras
            Self::build_virtual_camera_pipeline(&config, control_index),
            // Configuration 3: Alternative resolution for virtual cameras
            Self::build_scaled_pipeline(&config, control_index),
        ];

        let mut pipeline_started = false;

        for (config_idx, cmd) in pipeline_configs.iter().enumerate() {
            println!(
                "Trying pipeline configuration {} for camera {}",
                config_idx + 1,
                config.camera_index
            );

            if let Ok(mut child) = Command::new("cmd").args(&["/C", cmd]).spawn() {
                let cmd_pid = child.id();

                // Give it time to initialize
                std::thread::sleep(std::time::Duration::from_millis(2000));

                // Check if it's still running (success indicator)
                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        println!(
                            "Pipeline configuration {} exited early with status: {:?}",
                            config_idx + 1,
                            exit_status
                        );
                        continue; // Try next configuration
                    }
                    Ok(None) => {
                        println!(
                            "Pipeline configuration {} started successfully!",
                            config_idx + 1
                        );
                        pipeline_started = true;

                        // Find the actual gst-launch-1.0.exe process PID
                        Self::find_gstreamer_pid(cmd_pid, &gst_pid);

                        // Main loop - keep running until stopped
                        Self::monitor_pipeline(child, &streaming);
                        break; // Exit configuration loop
                    }
                    Err(e) => {
                        println!("Error checking pipeline status: {}", e);
                        continue; // Try next configuration
                    }
                }
            } else {
                println!("Failed to start pipeline configuration {}", config_idx + 1);
            }
        }

        if !pipeline_started {
            println!(
                "All pipeline configurations failed for camera {}",
                config.camera_index
            );
        }

        *gst_pid.lock().unwrap() = None;
    }

    fn build_standard_pipeline(config: &StreamConfig, control_index: usize) -> String {
        format!(
            "gst-launch-1.0 \
rtpbin name=rtp latency=150 \
fec-encoders=\"fec,0=\\\"raptorqenc\\ mtu\\=1356\\ symbol-size\\=1344\\ \
protected-packets\\=10\\ repair-packets\\=1000\\ repair-window\\=200\\\";\" \
ksvideosrc device-index={} ! \
videoconvert ! videorate ! video/x-raw,framerate=15/1,width=640,height=480 ! \
x264enc key-int-max=45 tune=zerolatency speed-preset=veryfast bitrate=2000 ! \
queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! mpegtsmux ! rtpmp2tpay ssrc={} ! \
rtp.send_rtp_sink_0 rtp.send_rtp_src_0 !
udpsink host={} port={} sync=false \
rtp.send_fec_src_0_0 ! udpsink host={} port={} async=false sync=false",
            config.camera_index, control_index, config.ip, config.port, config.ip, config.fec_port
        )
    }

    fn build_virtual_camera_pipeline(config: &StreamConfig, control_index: usize) -> String {
        format!(
            "gst-launch-1.0 \
rtpbin name=rtp latency=150 \
fec-encoders=\"fec,0=\\\"raptorqenc\\ mtu\\=1356\\ symbol-size\\=1344\\ \
protected-packets\\=10\\ repair-packets\\=1000\\ repair-window\\=200\\\";\" \
ksvideosrc device-index={} ! video/x-raw,width=640,height=480,framerate=30/1 ! \
videoconvert ! videorate ! video/x-raw,framerate=15/1 ! \
x264enc key-int-max=45 tune=zerolatency speed-preset=veryfast bitrate=2000 ! \
queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! mpegtsmux ! rtpmp2tpay ssrc={} ! \
rtp.send_rtp_sink_0 rtp.send_rtp_src_0 ! udpsink host={} port={} sync=false \
rtp.send_fec_src_0_0 ! udpsink host={} port={} async=false sync=false",
            config.camera_index, control_index, config.ip, config.port, config.ip, config.fec_port
        )
    }

    fn build_scaled_pipeline(config: &StreamConfig, control_index: usize) -> String {
        format!(
            "gst-launch-1.0 \
rtpbin name=rtp latency=150 \
fec-encoders=\"fec,0=\\\"raptorqenc\\ mtu\\=1356\\ symbol-size\\=1344\\ \
protected-packets\\=10\\ repair-packets\\=1000\\ repair-window\\=200\\\";\" \
ksvideosrc device-index={} ! video/x-raw,width=1280,height=720,framerate=30/1 ! \
videoconvert ! videoscale ! video/x-raw,width=640,height=480 ! videorate ! video/x-raw,framerate=15/1 ! \
x264enc key-int-max=45 tune=zerolatency speed-preset=veryfast bitrate=2000 ! \
queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! mpegtsmux ! rtpmp2tpay ssrc={} ! \
rtp.send_rtp_sink_0 rtp.send_rtp_src_0 ! udpsink host={} port={} sync=false \
rtp.send_fec_src_0_0 ! udpsink host={} port={} async=false sync=false",
            config.camera_index, control_index, config.ip, config.port, config.ip, config.fec_port
        )
    }

    fn find_gstreamer_pid(cmd_pid: u32, gst_pid: &Arc<Mutex<Option<u32>>>) {
        if let Ok(output) = Command::new("wmic")
            .args(&[
                "process",
                "where",
                &format!("ParentProcessId={}", cmd_pid),
                "get",
                "ProcessId",
                "/format:csv",
            ])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("ProcessId") && !line.contains("Node") {
                    if let Some(pid_str) = line.split(',').nth(1) {
                        if let Ok(pid) = pid_str.trim().parse::<u32>() {
                            *gst_pid.lock().unwrap() = Some(pid);
                            break;
                        }
                    }
                }
            }
        }
    }

    fn monitor_pipeline(mut child: std::process::Child, streaming: &Arc<Mutex<bool>>) {
        loop {
            if !*streaming.lock().unwrap() {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            match child.try_wait() {
                Ok(Some(_)) => {
                    println!("Pipeline exited unexpectedly");
                    break;
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(_) => break,
            }
        }
    }

    pub fn stop_pipeline(&mut self, control_index: usize) {
        *self.streaming[control_index].lock().unwrap() = false;

        if let Some(gst_process_pid) = *self.gst_pids[control_index].lock().unwrap() {
            let _ = Command::new("taskkill")
                .args(&["/F", "/PID", &gst_process_pid.to_string()])
                .output();
        } else {
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "gst-launch-1.0.exe"])
                .spawn();
        }

        if let Some(handle) = self.stream_threads[control_index].take() {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = handle.join();
        }
    }

    pub fn is_streaming(&self, control_index: usize) -> bool {
        *self.streaming[control_index].lock().unwrap()
    }
}
