#![allow(dead_code)]

use std::process::exit;

use app::DbMgrApp;
use bollard::Docker;
use data::read_config_file;
use iced::{Application, Font, Settings};

mod app;
mod data;
mod docker;

fn main() {
    let config = read_config_file();
    let docker = match Docker::connect_with_local_defaults() {
        Ok(val) => val,
        Err(ex) => {
            if let Err(dialog_err) = native_dialog::MessageDialog::new()
                .set_text(&format!("Error connecting to docker {ex}"))
                .set_type(native_dialog::MessageType::Error)
                .show_alert()
            {
                eprintln!("Application Error: {ex}");
                eprintln!("Dialog Error: {dialog_err}");
            }
            exit(-1);
        }
    };

    match DbMgrApp::run(Settings {
        id: None,
        antialiasing: true,
        default_font: Font::DEFAULT,
        default_text_size: 16.0,
        exit_on_close_request: true,
        window: iced::window::Settings {
            ..Default::default()
        },
        flags: (docker, config),
    }) {
        Ok(val) => val,
        Err(ex) => {
            if let Err(dialog_err) = native_dialog::MessageDialog::new()
                .set_text(&format!("Error running application {ex}"))
                .set_type(native_dialog::MessageType::Error)
                .show_alert()
            {
                eprintln!("Application Error: {ex}");
                eprintln!("Dialog Error: {dialog_err}");
            }
            exit(-1);
        }
    }
}
