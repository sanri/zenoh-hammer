mod app;
mod archive_file;
mod data_viewer;
mod hex_viewer;
mod language;
mod page_get;
mod page_put;
mod page_session;
mod page_sub;
mod payload_editor;
mod reply_viewer;
mod sample_viewer;
mod task_zenoh;
mod zenoh_data;

use directories::ProjectDirs;
use eframe::{
    egui::ViewportBuilder, icon_data::from_png_bytes, AppCreator, HardwareAcceleration,
    NativeOptions,
};
use env_logger::Env;
use log::{info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{app::HammerApp, language::load_fonts};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("zenoh_hammer=info")).init();

    let options = NativeOptions {
        hardware_acceleration: HardwareAcceleration::Required,
        viewport: ViewportBuilder {
            icon: Some(Arc::new(
                from_png_bytes(&include_bytes!("../media/hammer.png")[..]).unwrap(),
            )),
            ..ViewportBuilder::default()
        },
        ..NativeOptions::default()
    };

    let fonts = load_fonts();

    let mut hammer_app = HammerApp::default();

    let (acp, lof) = get_acp_lof();
    if let Some(app_config_path) = acp {
        hammer_app.set_app_config_path(app_config_path);
    }

    if let Some(last_opened_file) = lof {
        match hammer_app.load_from_file(last_opened_file.as_path()) {
            Ok(o) => {
                info!("{}", o);
                hammer_app.set_opened_file(last_opened_file);
            }
            Err(e) => {
                warn!("{}", e);
            }
        };
    }

    let create: AppCreator = Box::new(|cc| {
        cc.egui_ctx.set_fonts(fonts);
        Ok(Box::new(hammer_app))
    });
    let _ = eframe::run_native("Zenoh Hammer", options, create);
}

// 获取一个可读写的配置文件路径, 并读取最后一次打开文件的路径
// (app_config_path, last_opened_file)
fn get_acp_lof() -> (Option<PathBuf>, Option<PathBuf>) {
    if let Some((add, adp)) = app_data_path() {
        let adp_str = adp.to_string_lossy();
        let add_str = add.to_string_lossy();
        let file_name_str = adp
            .file_name()
            .map_or(String::new(), |name| name.to_string_lossy().to_string());

        if let Some(lof) = read_last_opened_file(adp.as_path()) {
            let lof_str = lof.to_string_lossy();
            if fs::write(adp.as_path(), lof_str.as_bytes()).is_ok() {
                info!("app data dir: \"{}\"", add_str);
                info!("app data file: \"{}\"", file_name_str);
                (Some(adp), Some(lof))
            } else {
                warn!("app data file are not writable. \"{adp_str}\"");
                (None, Some(lof))
            }
        } else {
            if fs::create_dir_all(add.as_path()).is_err() {
                warn!("the app data dir could not be created. \"{add_str}\"");
                return (None, None);
            }

            if fs::write(adp.as_path(), "    ").is_err() {
                warn!("app data file is not writable. \"{adp_str}\"");
                return (None, None);
            }

            info!("app data file is created. \"{}\"", adp_str);
            (Some(adp), None)
        }
    } else {
        (None, None)
    }
}

fn app_data_path() -> Option<(PathBuf, PathBuf)> {
    if let Some(proj_dir) = ProjectDirs::from("", "", "Zenoh Hammer") {
        let add = proj_dir.data_dir().to_path_buf();
        let mut adp = add.clone();
        adp.push("last_opened_file");
        Some((add, adp))
    } else {
        None
    }
}

fn read_last_opened_file(p: &Path) -> Option<PathBuf> {
    match fs::read_to_string(p) {
        Ok(o) => {
            let p = PathBuf::from(o);
            if p.is_file() {
                Some(p)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
