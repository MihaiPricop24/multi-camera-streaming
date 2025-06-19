use crate::types::StreamStats;
use gstreamer as gst;
use gstreamer::prelude::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct RealStatsCollector {
    pipeline: Option<gst::Pipeline>,
    stats: Arc<Mutex<StreamStats>>,
    running: Arc<Mutex<bool>>,
    thread_handle: Option<thread::JoinHandle<()>>,
    camera_index: usize,
}

impl RealStatsCollector {
    pub fn new(
        camera_index: usize,
        rtp_port: u16,
        fec_port: u16,
        stats: Arc<Mutex<StreamStats>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let _ = gst::init();

        let pipeline = Self::create_stats_pipeline(rtp_port, fec_port)?;

        Ok(Self {
            pipeline: Some(pipeline),
            stats,
            running: Arc::new(Mutex::new(false)),
            thread_handle: None,
            camera_index,
        })
    }

    fn create_stats_pipeline(
        rtp_port: u16,
        fec_port: u16,
    ) -> Result<gst::Pipeline, Box<dyn std::error::Error>> {
        let pipeline = gst::Pipeline::new();
        pipeline.set_name("real-stats-pipeline");

        // Create minimal pipeline to get raptor stats
        let rtp_src = gst::ElementFactory::make("udpsrc")?;
        let fec_src = gst::ElementFactory::make("udpsrc")?;
        let rtpbin = gst::ElementFactory::make("rtpbin")?;
        let jitterbuffer = gst::ElementFactory::make("rtpjitterbuffer")?;
        let fakesink = gst::ElementFactory::make("fakesink")?;

        // Set names
        rtp_src.set_name("real-rtp-src");
        fec_src.set_name("real-fec-src");
        rtpbin.set_name("real-rtpbin");
        jitterbuffer.set_name("real-jitterbuffer");
        fakesink.set_name("real-fakesink");

        // Configure UDP sources
        rtp_src.set_property("port", rtp_port as i32);
        fec_src.set_property("port", fec_port as i32);

        // Set caps
        let rtp_caps = gst::Caps::builder("application/x-rtp")
            .field("media", "video")
            .field("clock-rate", 90000i32)
            .field("encoding-name", "mp2t")
            .field("payload", 33i32)
            .build();
        rtp_src.set_property("caps", &rtp_caps);

        let fec_caps = gst::Caps::builder("application/x-rtp")
            .field("payload", 96i32)
            .field("raptor-scheme-id", "6")
            .field("repair-window", "200000")
            .field("t", "1344")
            .build();
        fec_src.set_property("caps", &fec_caps);

        // Configure rtpbin with FEC - this creates the raptorqdec with stats
        rtpbin.set_property("latency", 200u32);
        rtpbin.set_property_from_str(
            "fec-decoders",
            "fec,0=\"raptorqdec name=real-raptor repair-window-tolerance=200\";",
        );

        // Configure jitterbuffer
        jitterbuffer.set_property("latency", 600u32);
        jitterbuffer.set_property("do-lost", true);

        // Configure fakesink
        fakesink.set_property("sync", false);
        fakesink.set_property("silent", true);

        // Add elements
        pipeline.add_many(&[&rtp_src, &fec_src, &rtpbin, &jitterbuffer, &fakesink])?;

        // Link elements
        rtp_src.link_pads(Some("src"), &rtpbin, Some("recv_rtp_sink_0"))?;
        fec_src.link_pads(Some("src"), &rtpbin, Some("recv_fec_sink_0_0"))?;
        rtpbin.link_pads(Some("recv_rtp_src_0_0"), &jitterbuffer, Some("sink"))?;
        jitterbuffer.link(&fakesink)?;

        Ok(pipeline)
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        *self.running.lock().unwrap() = true;

        if let Some(pipeline) = &self.pipeline {
            pipeline.set_state(gst::State::Playing)?;

            let pipeline_weak = pipeline.downgrade();
            let stats = Arc::clone(&self.stats);
            let running = Arc::clone(&self.running);
            let camera_index = self.camera_index;

            self.thread_handle = Some(thread::spawn(move || {
                thread::sleep(Duration::from_millis(3000)); // Wait for pipeline

                while *running.lock().unwrap() {
                    if let Some(pipeline) = pipeline_weak.upgrade() {
                        Self::read_real_stats(&pipeline, &stats, camera_index);
                    }
                    thread::sleep(Duration::from_millis(2000));
                }
            }));
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;

        if let Some(pipeline) = &self.pipeline {
            let _ = pipeline.set_state(gst::State::Null);
        }

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    fn read_real_stats(
        pipeline: &gst::Pipeline,
        stats: &Arc<Mutex<StreamStats>>,
        camera_index: usize,
    ) {
        // Read the actual raptorqdec stats structure
        if let Some(raptor) = pipeline.by_name("real-raptor") {
            match raptor.property::<gst::Structure>("stats") {
                Ok(raptor_stats) => {
                    let mut stats_guard = stats.lock().unwrap();
                    let mut updated = false;

                    // Read received-packets
                    if let Ok(received) = raptor_stats.get::<u64>("received-packets") {
                        stats_guard.packets_received = received as u32;
                        updated = true;
                    }

                    // Read lost-packets
                    if let Ok(lost) = raptor_stats.get::<u64>("lost-packets") {
                        stats_guard.packets_lost = lost as u32;
                        updated = true;
                    }

                    // Read recovered-packets
                    if let Ok(recovered) = raptor_stats.get::<u64>("recovered-packets") {
                        stats_guard.packets_repaired = recovered as u32;
                        updated = true;
                    }

                    // Read buffered-media-packets (for bitrate estimation)
                    if let Ok(buffered_media) = raptor_stats.get::<u64>("buffered-media-packets") {
                        stats_guard.bitrate = (buffered_media as f32 * 0.1) + 800.0; // Rough estimate
                    }

                    // Read buffered-repair-packets (for latency estimation)
                    if let Ok(buffered_repair) = raptor_stats.get::<u64>("buffered-repair-packets")
                    {
                        stats_guard.latency = (buffered_repair as f32 * 2.0) + 10.0; // Rough estimate
                    }

                    if updated {
                        // Calculate repair rate
                        if stats_guard.packets_lost > 0 {
                            stats_guard.repair_rate = (stats_guard.packets_repaired as f32
                                / stats_guard.packets_lost as f32)
                                * 100.0;
                        } else if stats_guard.packets_repaired > 0 {
                            stats_guard.repair_rate = 100.0;
                        }

                        println!(
                            "Camera {} Real API Stats - Received:{}, Lost:{}, Recovered:{}, Rate:{:.1}%",
                            camera_index + 1,
                            stats_guard.packets_received,
                            stats_guard.packets_lost,
                            stats_guard.packets_repaired,
                            stats_guard.repair_rate
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "Camera {} - Failed to read raptor stats: {}",
                        camera_index + 1,
                        e
                    );
                }
            }
        } else {
            println!("Camera {} - raptor element not found", camera_index + 1);
        }

        // Also read jitterbuffer stats
        if let Some(jitterbuffer) = pipeline.by_name("real-jitterbuffer") {
            if let Ok(jb_stats) = jitterbuffer.property::<gst::Structure>("stats") {
                if let Ok(num_pushed) = jb_stats.get::<u64>("num-pushed") {
                    if let Ok(num_lost) = jb_stats.get::<u64>("num-lost") {
                        println!(
                            "Camera {} JitterBuffer - Pushed:{}, Lost:{}",
                            camera_index + 1,
                            num_pushed,
                            num_lost
                        );
                    }
                }
            }
        }
    }
}

impl Drop for RealStatsCollector {
    fn drop(&mut self) {
        self.stop();
    }
}
