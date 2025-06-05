mod backend;
mod gstreamer;
mod types;
mod ui;

use backend::CameraBackend;
use native_windows_gui as nwg;
use std::cell::RefCell;
use std::rc::Rc;
use ui::ReceiverUI;

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    // Create backend
    let backend = Rc::new(RefCell::new(CameraBackend::new()));

    // Create UI
    let mut ui = ReceiverUI::new(Rc::clone(&backend));
    ui.build().expect("Failed to build UI");

    let ui_rc = Rc::new(RefCell::new(ui));

    // Event handler
    let handler_ui = ui_rc.clone();
    let handler = move |evt, _evt_data, handle| match evt {
        nwg::Event::OnButtonClick => {
            let ui_ref = handler_ui.borrow();

            // Find which start button was clicked
            for i in 0..4 {
                if let Some(start_handle) = ui_ref.get_button_handle(i, "start") {
                    if handle == *start_handle {
                        drop(ui_ref);
                        handler_ui.borrow_mut().handle_start_button(i);
                        return;
                    }
                }
            }
        }
        nwg::Event::OnTimerTick => {
            let ui_ref = handler_ui.borrow();
            if handle == *ui_ref.get_timer_handle() {
                drop(ui_ref);
                handler_ui.borrow_mut().update_stats_display();
            }
        }
        nwg::Event::OnWindowClose => {
            handler_ui.borrow_mut().shutdown();
            nwg::stop_thread_dispatch();
        }
        _ => {}
    };

    nwg::bind_event_handler(
        ui_rc.borrow().get_window_handle(),
        ui_rc.borrow().get_window_handle(),
        handler,
    );
    nwg::dispatch_thread_events();
}
