use native_windows_gui as nwg;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Clone)]
pub struct CameraInfo {
    pub index: usize,
    pub name: String,
    pub device_path: String,
}

pub struct SenderApp {
    window: nwg::Window,

    // Dynamic camera controls
    camera_controls: Vec<CameraControls>,
    available_cameras: Vec<CameraInfo>,

    // UI Layout controls
    refresh_button: nwg::Button,
    camera_count_label: nwg::Label,

    // Headers
    ip_label: nwg::Label,
    port_label: nwg::Label,
    fec_label: nwg::Label,
    camera_label: nwg::Label,

    streaming: Vec<Arc<Mutex<bool>>>,
    stream_threads: Vec<Option<thread::JoinHandle<()>>>,
    gst_pids: Vec<Arc<Mutex<Option<u32>>>>,
}

pub struct CameraControls {
    pub camera_dropdown: nwg::ComboBox<String>,
    pub ip_input: nwg::TextInput,
    pub port_input: nwg::TextInput,
    pub fec_port_input: nwg::TextInput,
    pub start_button: nwg::Button,
    pub label: nwg::Label,
}

impl Default for CameraControls {
    fn default() -> Self {
        Self {
            camera_dropdown: Default::default(),
            ip_input: Default::default(),
            port_input: Default::default(),
            fec_port_input: Default::default(),
            start_button: Default::default(),
            label: Default::default(),
        }
    }
}

impl SenderApp {
    fn new() -> Self {
        Self {
            window: Default::default(),
            camera_controls: Vec::new(),
            available_cameras: Vec::new(),
            refresh_button: Default::default(),
            camera_count_label: Default::default(),
            ip_label: Default::default(),
            port_label: Default::default(),
            fec_label: Default::default(),
            camera_label: Default::default(),
            streaming: Vec::new(),
            stream_threads: Vec::new(),
            gst_pids: Vec::new(),
        }
    }

    fn detect_cameras(&mut self) -> Result<Vec<CameraInfo>, String> {
        let mut cameras = Vec::new();

        println!("Starting camera detection...");

        // Method 1: Test GStreamer directly for each device index with different caps
        for i in 0..6 {
            let mut camera_works = false;
            let mut device_name = format!("Camera Device {}", i);

            // Try different test pipelines for virtual cameras vs physical cameras
            let test_commands = vec![
                // Standard test
                format!(
                    "gst-launch-1.0 ksvideosrc device-index={} num-buffers=1 ! videoconvert ! fakesink",
                    i
                ),
                // With explicit caps for virtual cameras
                format!(
                    "gst-launch-1.0 ksvideosrc device-index={} ! video/x-raw,width=640,height=480,framerate=30/1 ! videoconvert ! fakesink",
                    i
                ),
                // Alternative caps
                format!(
                    "gst-launch-1.0 ksvideosrc device-index={} ! video/x-raw,width=1280,height=720,framerate=30/1 ! videoconvert ! fakesink",
                    i
                ),
                // Try with different formats
                format!(
                    "gst-launch-1.0 ksvideosrc device-index={} ! video/x-raw ! videoconvert ! fakesink",
                    i
                ),
            ];

            for (cmd_idx, test_cmd) in test_commands.iter().enumerate() {
                println!(
                    "Testing camera index {} with command {}: {}",
                    i,
                    cmd_idx + 1,
                    test_cmd
                );

                if let Ok(output) = Command::new("cmd").args(&["/C", test_cmd]).output() {
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    let stdout_str = String::from_utf8_lossy(&output.stdout);

                    println!(
                        "Index {} Command {} - Exit code: {}",
                        i,
                        cmd_idx + 1,
                        output.status.code().unwrap_or(-1)
                    );

                    if output.status.success()
                        || stderr_str.contains("Setting pipeline to PAUSED")
                        || stderr_str.contains("PREROLL")
                    {
                        camera_works = true;

                        // Try to get device name from error output
                        if stderr_str.contains("device-name") {
                            for line in stderr_str.lines() {
                                if line.contains("device-name") && line.contains("=") {
                                    if let Some(name_part) = line.split("device-name=").nth(1) {
                                        if let Some(clean_name) = name_part.split(',').next() {
                                            device_name = format!(
                                                "Index {}: {}",
                                                i,
                                                clean_name.trim_matches('"').trim()
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        // Check for Camo specifically
                        if stderr_str.contains("Camo") || device_name.contains("Camo") {
                            device_name = format!("Index {}: Camo Virtual Camera (iPhone)", i);
                        } else if stderr_str.contains("Integrated") || stderr_str.contains("USB") {
                            device_name = format!("Index {}: Integrated Camera", i);
                        }

                        println!(
                            "Found working camera: {} (using command {})",
                            device_name,
                            cmd_idx + 1
                        );
                        break;
                    } else {
                        println!("Index {} Command {} failed", i, cmd_idx + 1);
                        if stderr_str.contains("not-negotiated")
                            || stderr_str.contains("non negotiated")
                        {
                            println!("  -> Caps negotiation failed, trying next format...");
                        }
                    }
                } else {
                    println!("Failed to execute test command for index {}", i);
                }
            }

            if camera_works {
                cameras.push(CameraInfo {
                    index: i,
                    name: device_name,
                    device_path: format!("device-index={}", i),
                });
            } else {
                println!("Index {} - No working configuration found", i);
            }
        }

        // Method 2: Use PowerShell to get device names for reference
        if let Ok(output) = Command::new("powershell")
            .args(&[
                "-Command",
                "Get-WmiObject -Class Win32_PnPEntity | Where-Object { $_.PNPClass -eq 'Camera' -or $_.PNPClass -eq 'Image' -or $_.Name -like '*camera*' -or $_.Name -like '*webcam*' -or $_.Name -like '*camo*' } | Select-Object Name, DeviceID | Format-Table -AutoSize"
            ])
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                println!("PowerShell camera enumeration:");
                println!("{}", output_str);
            }
        }

        if cameras.is_empty() {
            println!("No cameras detected through any method");
        } else {
            println!("Total cameras detected: {}", cameras.len());
            for camera in &cameras {
                println!("  - Index {}: {}", camera.index, camera.name);
            }
        }

        self.available_cameras = cameras.clone();
        Ok(cameras)
    }

    fn build_ui(&mut self) -> Result<(), nwg::NwgError> {
        nwg::Window::builder()
            .size((750, 400))
            .position((300, 300))
            .title("Multi-Camera Sender with Auto-Detection")
            .build(&mut self.window)?;

        // Refresh button
        nwg::Button::builder()
            .text("Detect Cameras")
            .position((10, 10))
            .size((120, 30))
            .parent(&self.window)
            .build(&mut self.refresh_button)?;

        // Camera count label
        nwg::Label::builder()
            .text("No cameras detected")
            .position((140, 15))
            .size((200, 20))
            .parent(&self.window)
            .build(&mut self.camera_count_label)?;

        // Headers
        nwg::Label::builder()
            .text("Camera")
            .position((10, 50))
            .size((120, 20))
            .parent(&self.window)
            .build(&mut self.camera_label)?;

        nwg::Label::builder()
            .text("IP Address")
            .position((140, 50))
            .size((100, 20))
            .parent(&self.window)
            .build(&mut self.ip_label)?;

        nwg::Label::builder()
            .text("RTP Port")
            .position((250, 50))
            .size((80, 20))
            .parent(&self.window)
            .build(&mut self.port_label)?;

        nwg::Label::builder()
            .text("FEC Port")
            .position((340, 50))
            .size((80, 20))
            .parent(&self.window)
            .build(&mut self.fec_label)?;

        // Initially detect cameras
        if let Err(e) = self.detect_cameras() {
            nwg::simple_message("Error", &format!("Failed to detect cameras: {}", e));
        }

        self.create_camera_controls()?;
        self.update_camera_list();

        Ok(())
    }

    fn create_camera_controls(&mut self) -> Result<(), nwg::NwgError> {
        // Create controls for up to 6 cameras
        for i in 0..6 {
            let y_pos = 80 + (i * 40);
            let mut controls = CameraControls::default();

            // Camera dropdown
            nwg::ComboBox::builder()
                .position((10, y_pos))
                .size((120, 25))
                .parent(&self.window)
                .build(&mut controls.camera_dropdown)?;

            // IP input
            nwg::TextInput::builder()
                .text("192.168.0.101")
                .position((140, y_pos))
                .size((100, 25))
                .parent(&self.window)
                .build(&mut controls.ip_input)?;

            // Port input
            nwg::TextInput::builder()
                .text(&format!("{}", 5000 + i * 4))
                .position((250, y_pos))
                .size((80, 25))
                .parent(&self.window)
                .build(&mut controls.port_input)?;

            // FEC port input
            nwg::TextInput::builder()
                .text(&format!("{}", 5002 + i * 4))
                .position((340, y_pos))
                .size((80, 25))
                .parent(&self.window)
                .build(&mut controls.fec_port_input)?;

            // Start button
            nwg::Button::builder()
                .text("Start")
                .position((430, y_pos))
                .size((80, 25))
                .parent(&self.window)
                .enabled(false)
                .build(&mut controls.start_button)?;

            self.camera_controls.push(controls);
            self.streaming.push(Arc::new(Mutex::new(false)));
            self.stream_threads.push(None);
            self.gst_pids.push(Arc::new(Mutex::new(None)));
        }

        Ok(())
    }

    fn update_camera_list(&mut self) {
        // Update camera count label
        self.camera_count_label.set_text(&format!(
            "Detected {} camera(s)",
            self.available_cameras.len()
        ));

        // Update dropdowns
        for (i, controls) in self.camera_controls.iter_mut().enumerate() {
            // Clear existing items
            let mut items = Vec::new();

            if !self.available_cameras.is_empty() {
                items.push("Select Camera".to_string());
                for camera in &self.available_cameras {
                    items.push(camera.name.clone());
                }
                controls.start_button.set_enabled(true);
            } else {
                items.push("No cameras found".to_string());
                controls.start_button.set_enabled(false);
            }

            controls.camera_dropdown.set_collection(items);
            controls.camera_dropdown.set_selection(Some(0));
        }
    }

    fn get_selected_camera_index(&self, control_index: usize) -> Option<usize> {
        if let Some(selection) = self.camera_controls[control_index]
            .camera_dropdown
            .selection()
        {
            if selection > 0 && selection <= self.available_cameras.len() {
                return Some(self.available_cameras[selection - 1].index);
            }
        }
        None
    }

    fn start_pipeline(&mut self, control_index: usize) {
        let camera_device_index = match self.get_selected_camera_index(control_index) {
            Some(idx) => idx,
            None => {
                nwg::simple_message("Error", "Please select a camera first");
                return;
            }
        };

        let ip = self.camera_controls[control_index].ip_input.text();
        let port = self.camera_controls[control_index].port_input.text();
        let fec_port = self.camera_controls[control_index].fec_port_input.text();

        *self.streaming[control_index].lock().unwrap() = true;
        self.camera_controls[control_index]
            .start_button
            .set_text("Stop");

        let streaming = Arc::clone(&self.streaming[control_index]);
        let gst_pid = Arc::clone(&self.gst_pids[control_index]);

        self.stream_threads[control_index] = Some(thread::spawn(move || {
            // Try different pipeline configurations for virtual cameras
            let pipeline_configs = vec![
                // Configuration 1: Standard pipeline
                format!(
                    "gst-launch-1.0 \
    rtpbin name=rtp latency=150 \
    fec-encoders=\"fec,0=\\\"raptorqenc\\ mtu\\=1356\\ symbol-size\\=1344\\ \
    protected-packets\\=10\\ repair-packets\\=1000\\ repair-window\\=200\\\";\" \
    ksvideosrc device-index={} ! \
    videoconvert ! videorate ! video/x-raw,framerate=15/1,width=640,height=480 ! \
    x264enc key-int-max=45 tune=zerolatency speed-preset=veryfast bitrate=2000 ! \
    queue max-size-buffers=0 max-size-time=0 max-size-bytes=0 ! mpegtsmux ! rtpmp2tpay ssrc={} ! \
    rtp.send_rtp_sink_0 rtp.send_rtp_src_0 ! udpsink host={} port={} sync=false \
    rtp.send_fec_src_0_0 ! udpsink host={} port={} async=false sync=false",
                    camera_device_index, control_index, ip, port, ip, fec_port
                ),
                // Configuration 2: With explicit caps for virtual cameras
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
                    camera_device_index, control_index, ip, port, ip, fec_port
                ),
                // Configuration 3: Alternative resolution for virtual cameras
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
                    camera_device_index, control_index, ip, port, ip, fec_port
                ),
            ];

            let mut pipeline_started = false;

            for (config_idx, cmd) in pipeline_configs.iter().enumerate() {
                println!(
                    "Trying pipeline configuration {} for camera {}",
                    config_idx + 1,
                    camera_device_index
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

                            // Main loop - keep running until stopped
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
                                    Ok(None) => {
                                        std::thread::sleep(std::time::Duration::from_millis(100))
                                    }
                                    Err(_) => break,
                                }
                            }
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
                    camera_device_index
                );
            }

            *gst_pid.lock().unwrap() = None;
        }));
    }

    fn stop_pipeline(&mut self, control_index: usize) {
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

        self.camera_controls[control_index]
            .start_button
            .set_text("Start");

        if let Some(handle) = self.stream_threads[control_index].take() {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = handle.join();
        }
    }

    fn toggle_pipeline(&mut self, control_index: usize) {
        let is_streaming = *self.streaming[control_index].lock().unwrap();

        if is_streaming {
            self.stop_pipeline(control_index);
        } else {
            self.start_pipeline(control_index);
        }
    }

    fn refresh_cameras(&mut self) {
        if let Err(e) = self.detect_cameras() {
            nwg::simple_message("Error", &format!("Failed to detect cameras: {}", e));
        } else {
            self.update_camera_list();
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
            let app_ref = handler_app.borrow();

            // Check refresh button
            if handle == app_ref.refresh_button.handle {
                drop(app_ref);
                handler_app.borrow_mut().refresh_cameras();
                return;
            }

            // Check start buttons
            for (i, controls) in app_ref.camera_controls.iter().enumerate() {
                if handle == controls.start_button.handle {
                    drop(app_ref);
                    handler_app.borrow_mut().toggle_pipeline(i);
                    return;
                }
            }
        }
        nwg::Event::OnWindowClose => {
            {
                let app_ref = handler_app.borrow();
                for i in 0..app_ref.streaming.len() {
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
