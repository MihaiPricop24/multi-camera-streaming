use native_windows_gui as nwg;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Clone, Default)]
pub struct StreamStats {
    pub packets_received: u32,
    pub packets_lost: u32,
    pub packets_repaired: u32,
    pub repair_rate: f32,
    pub bitrate: f32,
    pub latency: f32,
}

pub struct ReceiverApp {
    window: nwg::Window,

    // Camera 1
    sender_ip_input_1: nwg::TextInput,
    rtp_port_input_1: nwg::TextInput,
    fec_port_input_1: nwg::TextInput,
    start_button_1: nwg::Button,
    stats_button_1: nwg::Button,

    // Camera 2
    sender_ip_input_2: nwg::TextInput,
    rtp_port_input_2: nwg::TextInput,
    fec_port_input_2: nwg::TextInput,
    start_button_2: nwg::Button,
    stats_button_2: nwg::Button,

    // Camera 3
    sender_ip_input_3: nwg::TextInput,
    rtp_port_input_3: nwg::TextInput,
    fec_port_input_3: nwg::TextInput,
    start_button_3: nwg::Button,
    stats_button_3: nwg::Button,

    // Camera 4
    sender_ip_input_4: nwg::TextInput,
    rtp_port_input_4: nwg::TextInput,
    fec_port_input_4: nwg::TextInput,
    start_button_4: nwg::Button,
    stats_button_4: nwg::Button,

    // Labels
    label_1: nwg::Label,
    label_2: nwg::Label,
    label_3: nwg::Label,
    label_4: nwg::Label,
    ip_label: nwg::Label,
    rtp_label: nwg::Label,
    fec_label: nwg::Label,

    receiving: Vec<Arc<Mutex<bool>>>,
    receive_threads: Vec<Option<thread::JoinHandle<()>>>,
    stats: Vec<Arc<Mutex<StreamStats>>>,
    gst_pids: Vec<Arc<Mutex<Option<u32>>>>,
}

impl ReceiverApp {
    fn new() -> Self {
        Self {
            window: Default::default(),

            sender_ip_input_1: Default::default(),
            rtp_port_input_1: Default::default(),
            fec_port_input_1: Default::default(),
            start_button_1: Default::default(),
            stats_button_1: Default::default(),

            sender_ip_input_2: Default::default(),
            rtp_port_input_2: Default::default(),
            fec_port_input_2: Default::default(),
            start_button_2: Default::default(),
            stats_button_2: Default::default(),

            sender_ip_input_3: Default::default(),
            rtp_port_input_3: Default::default(),
            fec_port_input_3: Default::default(),
            start_button_3: Default::default(),
            stats_button_3: Default::default(),

            sender_ip_input_4: Default::default(),
            rtp_port_input_4: Default::default(),
            fec_port_input_4: Default::default(),
            start_button_4: Default::default(),
            stats_button_4: Default::default(),

            label_1: Default::default(),
            label_2: Default::default(),
            label_3: Default::default(),
            label_4: Default::default(),
            ip_label: Default::default(),
            rtp_label: Default::default(),
            fec_label: Default::default(),

            receiving: vec![
                Arc::new(Mutex::new(false)),
                Arc::new(Mutex::new(false)),
                Arc::new(Mutex::new(false)),
                Arc::new(Mutex::new(false)),
            ],
            receive_threads: vec![None, None, None, None],
            stats: vec![
                Arc::new(Mutex::new(StreamStats::default())),
                Arc::new(Mutex::new(StreamStats::default())),
                Arc::new(Mutex::new(StreamStats::default())),
                Arc::new(Mutex::new(StreamStats::default())),
            ],
            gst_pids: vec![
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
                Arc::new(Mutex::new(None)),
            ],
        }
    }

    fn build_ui(&mut self) -> Result<(), nwg::NwgError> {
        nwg::Window::builder()
            .size((700, 350))
            .position((300, 300))
            .title("Multi-Camera Receiver")
            .build(&mut self.window)?;

        // Headers
        nwg::Label::builder()
            .text("Sender IP")
            .position((80, 10))
            .size((100, 20))
            .parent(&self.window)
            .build(&mut self.ip_label)?;

        nwg::Label::builder()
            .text("RTP Port")
            .position((200, 10))
            .size((80, 20))
            .parent(&self.window)
            .build(&mut self.rtp_label)?;

        nwg::Label::builder()
            .text("FEC Port")
            .position((300, 10))
            .size((80, 20))
            .parent(&self.window)
            .build(&mut self.fec_label)?;

        // Camera 1
        nwg::Label::builder()
            .text("Camera 1:")
            .position((10, 40))
            .size((60, 20))
            .parent(&self.window)
            .build(&mut self.label_1)?;

        nwg::TextInput::builder()
            .text("192.168.0.105")
            .position((80, 35))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.sender_ip_input_1)?;

        nwg::TextInput::builder()
            .text("5000")
            .position((200, 35))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.rtp_port_input_1)?;

        nwg::TextInput::builder()
            .text("5002")
            .position((300, 35))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.fec_port_input_1)?;

        nwg::Button::builder()
            .text("Start")
            .position((400, 35))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.start_button_1)?;

        nwg::Button::builder()
            .text("Stats")
            .position((500, 35))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.stats_button_1)?;

        // Camera 2
        nwg::Label::builder()
            .text("Camera 2:")
            .position((10, 80))
            .size((60, 20))
            .parent(&self.window)
            .build(&mut self.label_2)?;

        nwg::TextInput::builder()
            .text("192.168.0.105")
            .position((80, 75))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.sender_ip_input_2)?;

        nwg::TextInput::builder()
            .text("5004")
            .position((200, 75))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.rtp_port_input_2)?;

        nwg::TextInput::builder()
            .text("5006")
            .position((300, 75))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.fec_port_input_2)?;

        nwg::Button::builder()
            .text("Start")
            .position((400, 75))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.start_button_2)?;

        nwg::Button::builder()
            .text("Stats")
            .position((500, 75))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.stats_button_2)?;

        // Camera 3
        nwg::Label::builder()
            .text("Camera 3:")
            .position((10, 120))
            .size((60, 20))
            .parent(&self.window)
            .build(&mut self.label_3)?;

        nwg::TextInput::builder()
            .text("192.168.0.105")
            .position((80, 115))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.sender_ip_input_3)?;

        nwg::TextInput::builder()
            .text("5008")
            .position((200, 115))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.rtp_port_input_3)?;

        nwg::TextInput::builder()
            .text("5010")
            .position((300, 115))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.fec_port_input_3)?;

        nwg::Button::builder()
            .text("Start")
            .position((400, 115))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.start_button_3)?;

        nwg::Button::builder()
            .text("Stats")
            .position((500, 115))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.stats_button_3)?;

        // Camera 4
        nwg::Label::builder()
            .text("Camera 4:")
            .position((10, 160))
            .size((60, 20))
            .parent(&self.window)
            .build(&mut self.label_4)?;

        nwg::TextInput::builder()
            .text("192.168.0.105")
            .position((80, 155))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.sender_ip_input_4)?;

        nwg::TextInput::builder()
            .text("5012")
            .position((200, 155))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.rtp_port_input_4)?;

        nwg::TextInput::builder()
            .text("5014")
            .position((300, 155))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.fec_port_input_4)?;

        nwg::Button::builder()
            .text("Start")
            .position((400, 155))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.start_button_4)?;

        nwg::Button::builder()
            .text("Stats")
            .position((500, 155))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.stats_button_4)?;

        Ok(())
    }

    fn start_pipeline(&mut self, camera_index: usize) {
        let (_sender_ip, rtp_port, fec_port) = match camera_index {
            0 => (
                self.sender_ip_input_1.text(),
                self.rtp_port_input_1.text(),
                self.fec_port_input_1.text(),
            ),
            1 => (
                self.sender_ip_input_2.text(),
                self.rtp_port_input_2.text(),
                self.fec_port_input_2.text(),
            ),
            2 => (
                self.sender_ip_input_3.text(),
                self.rtp_port_input_3.text(),
                self.fec_port_input_3.text(),
            ),
            3 => (
                self.sender_ip_input_4.text(),
                self.rtp_port_input_4.text(),
                self.fec_port_input_4.text(),
            ),
            _ => return,
        };

        *self.receiving[camera_index].lock().unwrap() = true;

        let button = match camera_index {
            0 => &self.start_button_1,
            1 => &self.start_button_2,
            2 => &self.start_button_3,
            3 => &self.start_button_4,
            _ => return,
        };
        button.set_text("Stop");

        let receiving = Arc::clone(&self.receiving[camera_index]);
        let stats = Arc::clone(&self.stats[camera_index]);
        let gst_pid = Arc::clone(&self.gst_pids[camera_index]);

        self.receive_threads[camera_index] = Some(thread::spawn(move || {
            let cmd = format!(
                "gst-launch-1.0 --gst-debug=raptorqdec:5 \
                rtpbin latency=200 fec-decoders=\"fec,0=\\\"raptorqdec\\ repair-window-tolerance\\=200\\\";\" name=rtp \
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
                fec_port, rtp_port
            );

            if let Ok(mut child) = Command::new("cmd")
                .args(&["/C", &cmd])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                let cmd_pid = child.id();

                // Find the actual gst-launch-1.0.exe process PID
                std::thread::sleep(std::time::Duration::from_millis(1000)); // Give it time to start

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

                // Read stderr for GStreamer debug output
                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    let stats_clone = Arc::clone(&stats);

                    thread::spawn(move || {
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                parse_gstreamer_stats(&line, &stats_clone);
                            }
                        }
                    });
                }

                loop {
                    if !*receiving.lock().unwrap() {
                        // Kill the actual GStreamer process by PID
                        if let Some(gst_process_pid) = *gst_pid.lock().unwrap() {
                            let _ = Command::new("taskkill")
                                .args(&["/F", "/PID", &gst_process_pid.to_string()])
                                .output();
                        }

                        // Also kill the cmd process
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

                // Clear the PID
                *gst_pid.lock().unwrap() = None;
            }
        }));
    }

    fn stop_pipeline(&mut self, camera_index: usize) {
        *self.receiving[camera_index].lock().unwrap() = false;

        // Kill all GStreamer processes (reliable approach)
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", "gst-launch-1.0.exe"])
            .output(); // ← Use .output() instead of .spawn()

        let button = match camera_index {
            0 => &self.start_button_1,
            1 => &self.start_button_2,
            2 => &self.start_button_3,
            3 => &self.start_button_4,
            _ => return,
        };
        button.set_text("Start");

        // Wait for thread to finish and clean up
        if let Some(handle) = self.receive_threads[camera_index].take() {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = handle.join();
        }
    }

    fn toggle_pipeline(&mut self, camera_index: usize) {
        let is_receiving = *self.receiving[camera_index].lock().unwrap();

        if is_receiving {
            self.stop_pipeline(camera_index);
        } else {
            self.start_pipeline(camera_index);
        }
    }

    fn show_stats(&self, camera_index: usize) {
        let stats = self.stats[camera_index].lock().unwrap();
        let stats_text = format!(
            "Camera {} Statistics:\n\n\
            Packets Received: {}\n\
            Packets Lost: {}\n\
            Packets Repaired: {}\n\
            Repair Rate: {:.2}%\n\
            Bitrate: {:.2} kbps\n\
            Latency: {:.2} ms\n\
            \n\
            Loss Rate: {:.2}%\n\
            Recovery Rate: {:.2}%",
            camera_index + 1,
            stats.packets_received,
            stats.packets_lost,
            stats.packets_repaired,
            stats.repair_rate,
            stats.bitrate,
            stats.latency,
            if stats.packets_received > 0 {
                (stats.packets_lost as f32 / stats.packets_received as f32) * 100.0
            } else {
                0.0
            },
            if stats.packets_lost > 0 {
                (stats.packets_repaired as f32 / stats.packets_lost as f32) * 100.0
            } else {
                0.0
            }
        );

        nwg::simple_message("Statistics", &stats_text);
    }
}

fn parse_gstreamer_stats(line: &str, stats: &Arc<Mutex<StreamStats>>) {
    // Parse GStreamer debug output for RaptorQ statistics
    if line.contains("raptorqdec") {
        let mut stats_guard = stats.lock().unwrap();

        // Example parsing - adjust based on actual GStreamer output format
        if line.contains("packets-received") {
            if let Some(start) = line.find("packets-received=") {
                if let Ok(value) = line[start + 17..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<u32>()
                {
                    stats_guard.packets_received = value;
                }
            }
        }

        if line.contains("packets-lost") {
            if let Some(start) = line.find("packets-lost=") {
                if let Ok(value) = line[start + 13..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<u32>()
                {
                    stats_guard.packets_lost = value;
                }
            }
        }

        if line.contains("packets-repaired") {
            if let Some(start) = line.find("packets-repaired=") {
                if let Ok(value) = line[start + 17..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<u32>()
                {
                    stats_guard.packets_repaired = value;
                }
            }
        }

        if line.contains("repair-rate") {
            if let Some(start) = line.find("repair-rate=") {
                if let Ok(value) = line[start + 12..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("0")
                    .parse::<f32>()
                {
                    stats_guard.repair_rate = value;
                }
            }
        }
    }

    // Parse bitrate information
    if line.contains("bitrate") {
        let mut stats_guard = stats.lock().unwrap();
        if let Some(start) = line.find("bitrate=") {
            if let Ok(value) = line[start + 8..]
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse::<f32>()
            {
                stats_guard.bitrate = value / 1000.0; // Convert to kbps
            }
        }
    }

    // Parse latency information
    if line.contains("latency") {
        let mut stats_guard = stats.lock().unwrap();
        if let Some(start) = line.find("latency=") {
            if let Ok(value) = line[start + 8..]
                .split_whitespace()
                .next()
                .unwrap_or("0")
                .parse::<f32>()
            {
                stats_guard.latency = value / 1000000.0; // Convert to ms
            }
        }
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    let mut app = ReceiverApp::new();
    app.build_ui().expect("Failed to build UI");

    let app_rc = Rc::new(RefCell::new(app));

    let handler_app = app_rc.clone();
    let handler = move |evt, _evt_data, handle| match evt {
        nwg::Event::OnButtonClick => {
            let (start_camera_index, stats_camera_index) = {
                let app_ref = handler_app.borrow();
                let start_index = if handle == app_ref.start_button_1.handle {
                    Some(0)
                } else if handle == app_ref.start_button_2.handle {
                    Some(1)
                } else if handle == app_ref.start_button_3.handle {
                    Some(2)
                } else if handle == app_ref.start_button_4.handle {
                    Some(3)
                } else {
                    None
                };

                let stats_index = if handle == app_ref.stats_button_1.handle {
                    Some(0)
                } else if handle == app_ref.stats_button_2.handle {
                    Some(1)
                } else if handle == app_ref.stats_button_3.handle {
                    Some(2)
                } else if handle == app_ref.stats_button_4.handle {
                    Some(3)
                } else {
                    None
                };

                (start_index, stats_index)
            };

            if let Some(camera_index) = start_camera_index {
                handler_app.borrow_mut().toggle_pipeline(camera_index);
            } else if let Some(camera_index) = stats_camera_index {
                handler_app.borrow().show_stats(camera_index);
            }
        }
        nwg::Event::OnWindowClose => {
            {
                let app_ref = handler_app.borrow();
                for i in 0..4 {
                    *app_ref.receiving[i].lock().unwrap() = false;
                }
            }
            nwg::stop_thread_dispatch();
        }
        _ => {}
    };

    nwg::bind_event_handler(
        &app_rc.borrow().window.handle,
        &app_rc.borrow().window.handle,
        handler,
    );
    nwg::dispatch_thread_events();
}
