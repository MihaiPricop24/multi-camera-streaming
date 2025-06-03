use native_windows_gui as nwg;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub struct SenderApp {
    window: nwg::Window,

    // Camera 1
    ip_input_1: nwg::TextInput,
    port_input_1: nwg::TextInput,
    fec_port_input_1: nwg::TextInput,
    start_button_1: nwg::Button,

    // Camera 2
    ip_input_2: nwg::TextInput,
    port_input_2: nwg::TextInput,
    fec_port_input_2: nwg::TextInput,
    start_button_2: nwg::Button,

    // Camera 3
    ip_input_3: nwg::TextInput,
    port_input_3: nwg::TextInput,
    fec_port_input_3: nwg::TextInput,
    start_button_3: nwg::Button,

    // Camera 4
    ip_input_4: nwg::TextInput,
    port_input_4: nwg::TextInput,
    fec_port_input_4: nwg::TextInput,
    start_button_4: nwg::Button,

    // Labels
    label_1: nwg::Label,
    label_2: nwg::Label,
    label_3: nwg::Label,
    label_4: nwg::Label,
    ip_label: nwg::Label,
    port_label: nwg::Label,
    fec_label: nwg::Label,

    streaming: Vec<Arc<Mutex<bool>>>,
    stream_threads: Vec<Option<thread::JoinHandle<()>>>,
    gst_pids: Vec<Arc<Mutex<Option<u32>>>>,
}

impl SenderApp {
    fn new() -> Self {
        Self {
            window: Default::default(),

            ip_input_1: Default::default(),
            port_input_1: Default::default(),
            fec_port_input_1: Default::default(),
            start_button_1: Default::default(),

            ip_input_2: Default::default(),
            port_input_2: Default::default(),
            fec_port_input_2: Default::default(),
            start_button_2: Default::default(),

            ip_input_3: Default::default(),
            port_input_3: Default::default(),
            fec_port_input_3: Default::default(),
            start_button_3: Default::default(),

            ip_input_4: Default::default(),
            port_input_4: Default::default(),
            fec_port_input_4: Default::default(),
            start_button_4: Default::default(),

            label_1: Default::default(),
            label_2: Default::default(),
            label_3: Default::default(),
            label_4: Default::default(),
            ip_label: Default::default(),
            port_label: Default::default(),
            fec_label: Default::default(),

            streaming: vec![
                Arc::new(Mutex::new(false)),
                Arc::new(Mutex::new(false)),
                Arc::new(Mutex::new(false)),
                Arc::new(Mutex::new(false)),
            ],
            stream_threads: vec![None, None, None, None],
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
            .size((600, 350))
            .position((300, 300))
            .title("Multi-Camera Sender")
            .build(&mut self.window)?;

        // Headers
        nwg::Label::builder()
            .text("IP Address")
            .position((80, 10))
            .size((100, 20))
            .parent(&self.window)
            .build(&mut self.ip_label)?;

        nwg::Label::builder()
            .text("RTP Port")
            .position((200, 10))
            .size((80, 20))
            .parent(&self.window)
            .build(&mut self.port_label)?;

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
            .text("192.168.0.101")
            .position((80, 35))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.ip_input_1)?;

        nwg::TextInput::builder()
            .text("5000")
            .position((200, 35))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.port_input_1)?;

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

        // Camera 2
        nwg::Label::builder()
            .text("Camera 2:")
            .position((10, 80))
            .size((60, 20))
            .parent(&self.window)
            .build(&mut self.label_2)?;

        nwg::TextInput::builder()
            .text("192.168.0.101")
            .position((80, 75))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.ip_input_2)?;

        nwg::TextInput::builder()
            .text("5004")
            .position((200, 75))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.port_input_2)?;

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

        // Camera 3
        nwg::Label::builder()
            .text("Camera 3:")
            .position((10, 120))
            .size((60, 20))
            .parent(&self.window)
            .build(&mut self.label_3)?;

        nwg::TextInput::builder()
            .text("192.168.0.101")
            .position((80, 115))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.ip_input_3)?;

        nwg::TextInput::builder()
            .text("5008")
            .position((200, 115))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.port_input_3)?;

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

        // Camera 4
        nwg::Label::builder()
            .text("Camera 4:")
            .position((10, 160))
            .size((60, 20))
            .parent(&self.window)
            .build(&mut self.label_4)?;

        nwg::TextInput::builder()
            .text("192.168.0.101")
            .position((80, 155))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.ip_input_4)?;

        nwg::TextInput::builder()
            .text("5012")
            .position((200, 155))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.port_input_4)?;

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

        Ok(())
    }

    fn start_pipeline(&mut self, camera_index: usize) {
        let (ip, port, fec_port) = match camera_index {
            0 => (
                self.ip_input_1.text(),
                self.port_input_1.text(),
                self.fec_port_input_1.text(),
            ),
            1 => (
                self.ip_input_2.text(),
                self.port_input_2.text(),
                self.fec_port_input_2.text(),
            ),
            2 => (
                self.ip_input_3.text(),
                self.port_input_3.text(),
                self.fec_port_input_3.text(),
            ),
            3 => (
                self.ip_input_4.text(),
                self.port_input_4.text(),
                self.fec_port_input_4.text(),
            ),
            _ => return,
        };

        *self.streaming[camera_index].lock().unwrap() = true;

        let button = match camera_index {
            0 => &self.start_button_1,
            1 => &self.start_button_2,
            2 => &self.start_button_3,
            3 => &self.start_button_4,
            _ => return,
        };
        button.set_text("Stop");

        let streaming = Arc::clone(&self.streaming[camera_index]);
        let gst_pid = Arc::clone(&self.gst_pids[camera_index]);
        self.stream_threads[camera_index] = Some(thread::spawn(move || {
            let cmd = format!(
                "gst-launch-1.0 \
    rtpbin name=rtp latency=150 \
    fec-encoders=\"fec,0=\\\"raptorqenc\\ mtu\\=1356\\ symbol-size\\=1344\\ \
    protected-packets\\=10\\ repair-packets\\=1000\\ repair-window\\=200\\\";\" \
    ksvideosrc device-index=0 ! \
    videoconvert ! videorate ! video/x-raw,framerate=15/1,width=640,height=480 ! \
    x264enc key-int-max=45 tune=zerolatency speed-preset=veryfast bitrate=2000 ! \
    queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! mpegtsmux ! rtpmp2tpay ssrc={} ! \
    rtp.send_rtp_sink_0 rtp.send_rtp_src_0 ! udpsink host={} port={} sync=false \
    rtp.send_fec_src_0_0 ! udpsink host={} port={} async=false sync=false",
                camera_index, ip, port, ip, fec_port
            );

            if let Ok(mut child) = Command::new("cmd").args(&["/C", &cmd]).spawn() {
                let cmd_pid = child.id();

                std::thread::sleep(std::time::Duration::from_millis(1000));

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
                                    *gst_pid.lock().unwrap() = Some(pid); // ← STORE THE PID
                                    break;
                                }
                            }
                        }
                    }
                }

                loop {
                    if !*streaming.lock().unwrap() {
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
        }));
    }

    fn stop_pipeline(&mut self, camera_index: usize) {
        println!("Stop button clicked for camera {}", camera_index);

        *self.streaming[camera_index].lock().unwrap() = false;
        println!("Set streaming flag to false");

        // Check if we have a PID stored
        if let Some(gst_process_pid) = *self.gst_pids[camera_index].lock().unwrap() {
            println!(
                "Found stored PID: {}, attempting to kill...",
                gst_process_pid
            );
            let result = Command::new("taskkill")
                .args(&["/F", "/PID", &gst_process_pid.to_string()])
                .output();
            match result {
                Ok(output) => println!(
                    "Taskkill result: {}",
                    String::from_utf8_lossy(&output.stdout)
                ),
                Err(e) => println!("Taskkill failed: {}", e),
            }
        } else {
            println!("No PID stored, trying brute force kill of all gst-launch processes");
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "gst-launch-1.0.exe"])
                .spawn();
        }

        let button = match camera_index {
            0 => &self.start_button_1,
            1 => &self.start_button_2,
            2 => &self.start_button_3,
            3 => &self.start_button_4,
            _ => return,
        };
        button.set_text("Start");
        println!("Changed button text to Start");

        if let Some(handle) = self.stream_threads[camera_index].take() {
            println!("Waiting for thread to finish...");
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = handle.join();
            println!("Thread finished");
        }
    }

    fn toggle_pipeline(&mut self, camera_index: usize) {
        let is_streaming = *self.streaming[camera_index].lock().unwrap();

        if is_streaming {
            self.stop_pipeline(camera_index);
        } else {
            self.start_pipeline(camera_index);
        }
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    let mut app = SenderApp::new();
    app.build_ui().expect("Failed to build UI");

    let app_rc = Rc::new(RefCell::new(app));

    let handler_app = app_rc.clone();
    let handler = move |evt, _evt_data, handle| match evt {
        nwg::Event::OnButtonClick => {
            let camera_index = {
                let app_ref = handler_app.borrow();
                if handle == app_ref.start_button_1.handle {
                    Some(0)
                } else if handle == app_ref.start_button_2.handle {
                    Some(1)
                } else if handle == app_ref.start_button_3.handle {
                    Some(2)
                } else if handle == app_ref.start_button_4.handle {
                    Some(3)
                } else {
                    None
                }
            };

            if let Some(index) = camera_index {
                handler_app.borrow_mut().toggle_pipeline(index);
            }
        }
        nwg::Event::OnWindowClose => {
            {
                let app_ref = handler_app.borrow();
                for i in 0..4 {
                    *app_ref.streaming[i].lock().unwrap() = false;
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
