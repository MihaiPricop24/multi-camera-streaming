use crate::types::CameraConfig;
use crate::stats_collector::StatsCollector;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct GStreamerPipeline {
    camera_index: usize,
    config: CameraConfig,
    receiving: Arc<Mutex<bool>>,
    stats_collector: Option<Arc<Mutex<StatsCollector>>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl GStreamerPipeline {
    pub fn new(
        camera_index: usize,
        config: CameraConfig,
        receiving: Arc<Mutex<bool>>,
        stats_collector: Option<Arc<Mutex<StatsCollector>>>,
    ) -> Self {
        Self {
            camera_index,
            config,
            receiving,
            stats_collector,
            thread_handle: None,
        }
    }

    pub fn start(&mut self) {
        *self.receiving.lock().unwrap() = true;

        let camera_index = self.camera_index;
        let config = self.config.clone();
        let receiving = Arc::clone(&self.receiving);
        let stats_collector = self.stats_collector.clone();

        self.thread_handle = Some(thread::spawn(move || {
            run_pipeline(camera_index, config, receiving, stats_collector);
        }));
    }

    pub fn stop(&mut self) {
        *self.receiving.lock().unwrap() = false;

        // Kill all GStreamer processes
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "gst-launch-1.0.exe"])
            .output();

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = handle.join();
        }
    }
}

fn run_pipeline(
    camera_index: usize,
    config: CameraConfig,
    receiving: Arc<Mutex<bool>>,
    stats_collector: Option<Arc<Mutex<StatsCollector>>>,
) {
    let cmd = format!(
        "gst-launch-1.0 --gst-debug=raptorqdec:5,rtpjitterbuffer:4 \
        rtpbin latency=200 \
        fec-decoders=\"fec,0=\\\"raptorqdec\\ name=raptor_{}\\ \
        repair-window-tolerance\\=200\\\";\" name=rtp \
        udpsrc port={} address=0.0.0.0 \
        caps=\"application/x-rtp, payload=96, raptor-scheme-id=(string)6, \
        repair-window=(string)200000, t=(string)1344\" ! \
        queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! rtp.recv_fec_sink_0_0 \
        udpsrc port={} address=0.0.0.0 \
        caps=\"application/x-rtp, media=video, clock-rate=90000, \
        encoding-name=mp2t, payload=33\" ! \
        queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! \
        netsim drop-probability=0.5 duplicate-probability=0.1 delay-distribution=normal ! \
        rtp.recv_rtp_sink_0 \
        rtp. ! rtpjitterbuffer latency=600 do-lost=true ! rtpmp2tdepay ! \
        tsdemux ! h264parse ! avdec_h264 max-threads=4 ! videoconvert ! videorate ! \
        video/x-raw,framerate=15/1 ! autovideosink sync=false",
        camera_index,
        config.fec_port,
        config.rtp_port
    );

    println!("Camera {} - Starting GStreamer with debug output capture", camera_index + 1);

    if let Ok(mut child) = Command::new("cmd")
        .args(["/C", &cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        // Capture and parse stderr for debug output
        if let Some(stderr) = child.stderr.take() {
            let receiving_clone = Arc::clone(&receiving);
            let stats_collector_clone = stats_collector.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if !*receiving_clone.lock().unwrap() {
                        break;
                    }
                    
                    if let Ok(line_content) = line {
                        // Only process lines with our target debug info
                        if line_content.contains("Successfully recovered packet") || 
                           line_content.contains("Add Lost timer for #") {
                            
                            // Send to stats collector if available
                            if let Some(ref stats_collector_arc) = stats_collector_clone {
                                if let Ok(mut stats_collector) = stats_collector_arc.lock() {
                                    stats_collector.parse_debug_line(&line_content);
                                }
                            }
                        }
                    }
                }
            });
        }

        // Main pipeline monitoring loop
        loop {
            if !*receiving.lock().unwrap() {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(_) => break,
            }
        }
    } else {
        println!("Camera {} - Failed to start GStreamer pipeline", camera_index + 1);
    }
}