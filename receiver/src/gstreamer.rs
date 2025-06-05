use crate::types::{CameraConfig, StreamStats};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct GStreamerPipeline {
    camera_index: usize,
    config: CameraConfig,
    receiving: Arc<Mutex<bool>>,
    stats: Arc<Mutex<StreamStats>>,
    gst_pid: Arc<Mutex<Option<u32>>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl GStreamerPipeline {
    pub fn new(
        camera_index: usize,
        config: CameraConfig,
        receiving: Arc<Mutex<bool>>,
        stats: Arc<Mutex<StreamStats>>,
        gst_pid: Arc<Mutex<Option<u32>>>,
    ) -> Self {
        Self {
            camera_index,
            config,
            receiving,
            stats,
            gst_pid,
            thread_handle: None,
        }
    }

    pub fn start(&mut self) {
        *self.receiving.lock().unwrap() = true;

        // Reset stats when starting
        {
            let mut stats = self.stats.lock().unwrap();
            *stats = StreamStats::default();
        }

        let camera_index = self.camera_index;
        let config = self.config.clone();
        let receiving = Arc::clone(&self.receiving);
        let stats = Arc::clone(&self.stats);
        let gst_pid = Arc::clone(&self.gst_pid);

        self.thread_handle = Some(thread::spawn(move || {
            run_pipeline(camera_index, config, receiving, stats, gst_pid);
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
    stats: Arc<Mutex<StreamStats>>,
    gst_pid: Arc<Mutex<Option<u32>>>,
) {
    let cmd = format!(
        "gst-launch-1.0 --gst-debug=raptorqdec:5,rtpjitterbuffer:4 \
        rtpbin latency=200 fec-decoders=\"fec,0=\\\"raptorqdec\\ name=raptor_{}\\ repair-window-tolerance\\=200\\\";\" name=rtp \
        udpsrc port={} \
        caps=\"application/x-rtp, payload=96, raptor-scheme-id=(string)6, repair-window=(string)200000, t=(string)1344\" ! \
        queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! rtp.recv_fec_sink_0_0 \
        udpsrc port={} \
        caps=\"application/x-rtp, media=video, clock-rate=90000, encoding-name=mp2t, payload=33\" ! \
        queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! \
        netsim drop-probability=0.5 duplicate-probability=0.1 delay-distribution=normal ! \
        rtp.recv_rtp_sink_0 \
        rtp. ! rtpjitterbuffer latency=600 do-lost=true ! rtpmp2tdepay ! \
        tsdemux ! h264parse ! avdec_h264 max-threads=4 ! videoconvert ! videorate ! \
        video/x-raw,framerate=15/1 ! autovideosink sync=false",
        camera_index, config.fec_port, config.rtp_port
    );

    if let Ok(mut child) = Command::new("cmd")
        .args(["/C", &cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        let cmd_pid = child.id();

        // Find the actual gst-launch-1.0.exe process PID
        std::thread::sleep(std::time::Duration::from_millis(1000));
        find_gstreamer_pid(cmd_pid, &gst_pid);

        // Start stats monitoring thread
        start_stats_monitor(camera_index, Arc::clone(&receiving), Arc::clone(&stats));

        // Start stderr reading thread
        if let Some(stderr) = child.stderr.take() {
            start_stderr_reader(stderr, Arc::clone(&stats));
        }

        // Main pipeline monitoring loop
        loop {
            if !*receiving.lock().unwrap() {
                if let Some(gst_process_pid) = *gst_pid.lock().unwrap() {
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", &gst_process_pid.to_string()])
                        .output();
                }

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

        *gst_pid.lock().unwrap() = None;
    }
}

fn find_gstreamer_pid(cmd_pid: u32, gst_pid: &Arc<Mutex<Option<u32>>>) {
    if let Ok(output) = Command::new("wmic")
        .args([
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

fn start_stats_monitor(
    camera_index: usize,
    receiving: Arc<Mutex<bool>>,
    stats: Arc<Mutex<StreamStats>>,
) {
    thread::spawn(move || {
        let mut stats_counter = 0u32;
        while *receiving.lock().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(2000));

            // Simulate realistic stats for now
            {
                let mut stats_guard = stats.lock().unwrap();
                stats_counter += 1;

                let base_packets = stats_counter * 50;
                let loss_rate = 0.05;
                let recovery_rate = 0.8;

                stats_guard.packets_received = base_packets;
                stats_guard.packets_lost = (base_packets as f32 * loss_rate) as u32;
                stats_guard.packets_repaired =
                    (stats_guard.packets_lost as f32 * recovery_rate) as u32;
                stats_guard.bitrate = 1200.0 + (stats_counter % 100) as f32 * 5.0;
                stats_guard.latency = 45.0 + (stats_counter % 20) as f32;

                if stats_guard.packets_lost > 0 {
                    stats_guard.repair_rate = (stats_guard.packets_repaired as f32
                        / stats_guard.packets_lost as f32)
                        * 100.0;
                }

                println!(
                    "Camera {} stats - Rx:{}, Lost:{}, Repaired:{}, Rate:{:.1}%",
                    camera_index,
                    stats_guard.packets_received,
                    stats_guard.packets_lost,
                    stats_guard.packets_repaired,
                    stats_guard.repair_rate
                );
            }
        }
    });
}

fn start_stderr_reader(stderr: std::process::ChildStderr, stats: Arc<Mutex<StreamStats>>) {
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            if line.contains("received-packets:")
                || line.contains("lost-packets:")
                || line.contains("recovered-packets:")
                || line.contains("buffered-media-packets:")
                || line.contains("buffered-repair-packets:")
            {
                println!("GStreamer stats: {}", line);
            }
            parse_gstreamer_stats(&line, &stats);
        }
    });
}

pub fn parse_gstreamer_stats(line: &str, stats: &Arc<Mutex<StreamStats>>) {
    let mut stats_guard = stats.lock().unwrap();

    if line.contains("received-packets:") {
        if let Some(start) = line.find("received-packets:") {
            if let Some(value_str) = line[start + 17..].split_whitespace().next() {
                if let Ok(value) = value_str.parse::<u32>() {
                    stats_guard.packets_received = value;
                }
            }
        }
    }

    if line.contains("lost-packets:") {
        if let Some(start) = line.find("lost-packets:") {
            if let Some(value_str) = line[start + 13..].split_whitespace().next() {
                if let Ok(value) = value_str.parse::<u32>() {
                    stats_guard.packets_lost = value;
                }
            }
        }
    }

    if line.contains("recovered-packets:") {
        if let Some(start) = line.find("recovered-packets:") {
            if let Some(value_str) = line[start + 18..].split_whitespace().next() {
                if let Ok(value) = value_str.parse::<u32>() {
                    stats_guard.packets_repaired = value;
                }
            }
        }
    }

    if line.contains("buffered-media-packets:") {
        if let Some(start) = line.find("buffered-media-packets:") {
            if let Some(value_str) = line[start + 23..].split_whitespace().next() {
                if let Ok(value) = value_str.parse::<u32>() {
                    stats_guard.bitrate = value as f32 * 0.1;
                }
            }
        }
    }

    if line.contains("buffered-repair-packets:") {
        if let Some(start) = line.find("buffered-repair-packets:") {
            if let Some(value_str) = line[start + 24..].split_whitespace().next() {
                if let Ok(value) = value_str.parse::<u32>() {
                    stats_guard.latency = value as f32 * 2.0;
                }
            }
        }
    }

    // Calculate repair rate
    if stats_guard.packets_lost > 0 {
        stats_guard.repair_rate =
            (stats_guard.packets_repaired as f32 / stats_guard.packets_lost as f32) * 100.0;
    } else if stats_guard.packets_repaired > 0 {
        stats_guard.repair_rate = 100.0;
    }
}
