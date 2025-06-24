use native_windows_gui as nwg;
use std::cell::RefCell;
use std::rc::Rc;

mod backend;
mod gstreamer;
mod types;
mod ui;

use ui::SenderApp;

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    let mut app = SenderApp::new();
    app.build_ui().expect("Failed to build UI");

    let app_rc = Rc::new(RefCell::new(app));

    let handler_app = app_rc.clone();
    let handler = move |evt, _evt_data, handle| match evt {
        nwg::Event::OnButtonClick => {
            let app_ref = handler_app.borrow();

            if handle == app_ref.refresh_button.handle {
                drop(app_ref);
                handler_app.borrow_mut().refresh_cameras();
                return;
            }

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
