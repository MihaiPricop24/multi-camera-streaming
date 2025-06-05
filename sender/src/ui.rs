// ui.rs
use crate::backend::CameraBackend;
use crate::gstreamer::GStreamerManager;
use crate::types::{CameraInfo, StreamConfig};
use native_windows_gui as nwg;
use std::sync::{Arc, Mutex};

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

pub struct SenderApp {
    pub window: nwg::Window,
    pub camera_controls: Vec<CameraControls>,
    available_cameras: Vec<CameraInfo>,
    pub refresh_button: nwg::Button,
    camera_count_label: nwg::Label,
    ip_label: nwg::Label,
    port_label: nwg::Label,
    fec_label: nwg::Label,
    camera_label: nwg::Label,
    gstreamer_manager: GStreamerManager,
    pub streaming: Vec<Arc<Mutex<bool>>>,
}

impl SenderApp {
    pub fn new() -> Self {
        let mut gstreamer_manager = GStreamerManager::new();
        gstreamer_manager.initialize_streams(6);

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
            streaming: gstreamer_manager.streaming.clone(),
            gstreamer_manager,
        }
    }

    pub fn build_ui(&mut self) -> Result<(), nwg::NwgError> {
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

        self.build_headers()?;

        // Initially detect cameras - this should happen BEFORE creating controls
        if let Err(e) = self.detect_cameras() {
            println!(
                "Warning: Failed to detect cameras during initialization: {}",
                e
            );
            // Don't return error, just log it and continue
        }

        self.create_camera_controls()?;
        self.update_camera_list();

        Ok(())
    }

    fn build_headers(&mut self) -> Result<(), nwg::NwgError> {
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

        Ok(())
    }

    fn detect_cameras(&mut self) -> Result<(), String> {
        match CameraBackend::detect_cameras() {
            Ok(cameras) => {
                self.available_cameras = cameras;
                Ok(())
            }
            Err(e) => {
                println!("Camera detection error: {}", e);
                self.available_cameras = Vec::new(); // Ensure it's empty on error
                Err(e)
            }
        }
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

            // Note: The label field in CameraControls isn't used in the original code
            // but we need to initialize it to avoid compilation errors
            nwg::Label::builder()
                .text("")
                .position((520, y_pos))
                .size((50, 25))
                .parent(&self.window)
                .build(&mut controls.label)?;

            self.camera_controls.push(controls);
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
        for controls in self.camera_controls.iter_mut() {
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

    pub fn toggle_pipeline(&mut self, control_index: usize) {
        let is_streaming = self.gstreamer_manager.is_streaming(control_index);

        if is_streaming {
            self.gstreamer_manager.stop_pipeline(control_index);
            self.camera_controls[control_index]
                .start_button
                .set_text("Start");
        } else {
            let camera_device_index = match self.get_selected_camera_index(control_index) {
                Some(idx) => idx,
                None => {
                    nwg::simple_message("Error", "Please select a camera first");
                    return;
                }
            };

            let config = StreamConfig {
                camera_index: camera_device_index,
                ip: self.camera_controls[control_index].ip_input.text(),
                port: self.camera_controls[control_index].port_input.text(),
                fec_port: self.camera_controls[control_index].fec_port_input.text(),
            };

            self.gstreamer_manager.start_pipeline(control_index, config);
            self.camera_controls[control_index]
                .start_button
                .set_text("Stop");
        }
    }

    pub fn refresh_cameras(&mut self) {
        if let Err(e) = self.detect_cameras() {
            nwg::simple_message("Error", &format!("Failed to detect cameras: {}", e));
        } else {
            self.update_camera_list();
        }
    }

    // Add cleanup method for when the app closes
    pub fn cleanup(&mut self) {
        // Stop all streaming before cleanup
        for i in 0..self.streaming.len() {
            if self.gstreamer_manager.is_streaming(i) {
                self.gstreamer_manager.stop_pipeline(i);
            }
        }
    }
}
