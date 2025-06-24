use crate::backend::CameraBackend;
use native_windows_gui as nwg;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ReceiverUI {
    window: nwg::Window,

    sender_ip_inputs: Vec<nwg::TextInput>,
    rtp_port_inputs: Vec<nwg::TextInput>,
    fec_port_inputs: Vec<nwg::TextInput>,
    start_buttons: Vec<nwg::Button>,

    camera_labels: Vec<nwg::Label>,
    stats_displays: Vec<nwg::Label>,
    header_labels: Vec<nwg::Label>,

    stats_timer: nwg::AnimationTimer,

    backend: Rc<RefCell<CameraBackend>>,
}

impl ReceiverUI {
    pub fn new(backend: Rc<RefCell<CameraBackend>>) -> Self {
        Self {
            window: Default::default(),
            sender_ip_inputs: vec![
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            rtp_port_inputs: vec![
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            fec_port_inputs: vec![
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            start_buttons: vec![
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            camera_labels: vec![
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            stats_displays: vec![
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            header_labels: vec![
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            stats_timer: Default::default(),
            backend,
        }
    }

    pub fn build(&mut self) -> Result<(), nwg::NwgError> {
        nwg::Window::builder()
            .size((1400, 280))
            .position((300, 300))
            .title("Multi-Camera Receiver with REAL Stats")
            .build(&mut self.window)?;

        self.build_headers()?;

        for i in 0..4 {
            self.build_camera_row(i)?;
        }

        nwg::AnimationTimer::builder()
            .parent(&self.window)
            .interval(std::time::Duration::from_millis(1000))
            .build(&mut self.stats_timer)?;

        Ok(())
    }

    fn build_headers(&mut self) -> Result<(), nwg::NwgError> {
        nwg::Label::builder()
            .text("Sender IP")
            .position((80, 10))
            .size((100, 20))
            .parent(&self.window)
            .build(&mut self.header_labels[0])?;

        nwg::Label::builder()
            .text("RTP Port")
            .position((200, 10))
            .size((80, 20))
            .parent(&self.window)
            .build(&mut self.header_labels[1])?;

        nwg::Label::builder()
            .text("FEC Port")
            .position((300, 10))
            .size((80, 20))
            .parent(&self.window)
            .build(&mut self.header_labels[2])?;

        nwg::Label::builder()
            .text("REAL Stream Statistics")
            .position((500, 10))
            .size((800, 20))
            .parent(&self.window)
            .build(&mut self.header_labels[3])?;

        Ok(())
    }

    fn build_camera_row(&mut self, camera_index: usize) -> Result<(), nwg::NwgError> {
        let y_pos = (40 + camera_index * 60) as i32;
        let config = self
            .backend
            .borrow()
            .get_camera_config(camera_index)
            .unwrap()
            .clone();

        nwg::Label::builder()
            .text(&format!("Camera {}:", camera_index + 1))
            .position((10, y_pos))
            .size((60, 20))
            .parent(&self.window)
            .build(&mut self.camera_labels[camera_index])?;

        nwg::TextInput::builder()
            .text(&config.sender_ip)
            .position((80, y_pos - 5))
            .size((100, 25))
            .parent(&self.window)
            .build(&mut self.sender_ip_inputs[camera_index])?;

        nwg::TextInput::builder()
            .text(&config.rtp_port)
            .position((200, y_pos - 5))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.rtp_port_inputs[camera_index])?;

        nwg::TextInput::builder()
            .text(&config.fec_port)
            .position((300, y_pos - 5))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.fec_port_inputs[camera_index])?;

        nwg::Button::builder()
            .text("Start")
            .position((400, y_pos - 5))
            .size((80, 25))
            .parent(&self.window)
            .build(&mut self.start_buttons[camera_index])?;

        nwg::Label::builder()
            .text("Waiting for stream...")
            .position((500, y_pos - 5))
            .size((880, 25))
            .parent(&self.window)
            .build(&mut self.stats_displays[camera_index])?;

        Ok(())
    }

    pub fn update_stats_display(&mut self) {
        for i in 0..4 {
            if self.backend.borrow().is_camera_running(i) {
                if let Some(stats) = self.backend.borrow().get_camera_stats(i) {
                    let stats_text = format!(
                        "Received:{} Lost:{} Late:{} Sent:{} Repair:{:.1}% Bitrate:{:.1}kbps Latency:{:.1}ms",
                        stats.packets_received,
                        stats.packets_lost,
                        stats.packets_late,
                        stats.packets_sent,
                        stats.repair_rate,
                        stats.bitrate,
                        stats.latency
                    );
                    self.stats_displays[i].set_text(&stats_text);
                } else {
                    self.stats_displays[i].set_text("Collecting stats...");
                }
            } else {
                self.stats_displays[i].set_text("Waiting for stream...");
            }
        }
    }

    pub fn handle_start_button(&mut self, camera_index: usize) {
        let sender_ip = self.sender_ip_inputs[camera_index].text();
        let rtp_port = self.rtp_port_inputs[camera_index].text();
        let fec_port = self.fec_port_inputs[camera_index].text();

        self.backend.borrow_mut().update_camera_config(
            camera_index,
            &sender_ip,
            &rtp_port,
            &fec_port,
        );

        if let Err(e) = self.backend.borrow_mut().toggle_camera(camera_index) {
            nwg::simple_message(
                "Error",
                &format!("Failed to toggle camera {}: {}", camera_index + 1, e),
            );
        }

        let button_text = if self.backend.borrow().is_camera_running(camera_index) {
            "Stop"
        } else {
            "Start"
        };
        self.start_buttons[camera_index].set_text(button_text);

        let any_running = (0..4).any(|i| self.backend.borrow().is_camera_running(i));
        if any_running {
            self.stats_timer.start();
        } else {
            self.stats_timer.stop();
        }
    }

    pub fn get_window_handle(&self) -> &nwg::ControlHandle {
        &self.window.handle
    }

    pub fn get_button_handle(
        &self,
        camera_index: usize,
        button_type: &str,
    ) -> Option<&nwg::ControlHandle> {
        match button_type {
            "start" => Some(&self.start_buttons[camera_index].handle),
            _ => None,
        }
    }

    pub fn get_timer_handle(&self) -> &nwg::ControlHandle {
        &self.stats_timer.handle
    }

    pub fn shutdown(&mut self) {
        self.backend.borrow_mut().shutdown();
    }
}
