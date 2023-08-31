pub mod build;
pub mod choices;
pub mod download;
pub mod log;
pub mod process;
pub mod tui;
pub mod ui;
pub mod user;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use build::build_thread;

use process::run_process;
use tui::Tui;
use ui::{TickResult, UI};

fn main() {
    // This program is rather CPU intensive when idle (30-60% singlecore), so adjust niceness, so
    // building takes priority

    let mut tui = Tui::new().unwrap();
    let mut ui = Arc::new(Mutex::new(UI::new()));

    run_process(
        format!("renice -n 10 -p {}", std::process::id()).as_str(),
        &mut ui,
    )
    .unwrap();

    let thread_ui = ui.clone();
    let _build_thread = thread::spawn(move || build_thread(thread_ui.clone()));

    // Main loop
    loop {
        let tick_result = ui.lock().unwrap().tick();
        match tick_result {
            Ok(TickResult::Exit) => {
                break;
            }
            Ok(_) => {}
            Err(err) => {
                println!("Err: {}", err);
                break;
            }
        }

        let res = tui.draw(|frame| {
            ui.lock().unwrap().render(frame);
        });

        if let Some(err) = res.err() {
            println!("Err: {}", err);
            break;
        }
        // Avoid wasting CPU
        thread::sleep(Duration::from_millis(10));
    }
}
