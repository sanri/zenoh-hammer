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

use crate::{app::HammerApp, language::load_fonts};
use directories::ProjectDirs;
use eframe::{
    egui::ViewportBuilder, icon_data::from_png_bytes, AppCreator, HardwareAcceleration,
    NativeOptions,
};
use env_logger::Env;
use image::imageops::index_colors;
use log::{info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

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
        match hammer_app.load_from_file(last_opened_file.clone()) {
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
    if let Some((acd, acp)) = app_config_path() {
        let acp_str = acp.to_string_lossy();

        if let Some(lof) = read_last_opened_file(acp.as_path()) {
            let lof_str = lof.to_string_lossy();
            if fs::write(acp.as_path(), lof_str.as_bytes()).is_ok() {
                info!("Default config file: {}", acp_str);
                (Some(acp), Some(lof))
            } else {
                warn!("default config file are not writable. \"{acp_str}\"");
                (None, Some(lof))
            }
        } else {
            if fs::create_dir_all(acd.as_path()).is_err() {
                let acd_str = acd.to_string_lossy();
                warn!("the default config dir could not be created. \"{acd_str}\"");
                return (None, None);
            }

            if fs::write(acp.as_path(), "    ").is_err() {
                warn!("default config file is not writable. \"{acp_str}\"");
                return (None, None);
            }

            info!("default config file is created: {}", acp_str);
            (Some(acp), None)
        }
    } else {
        (None, None)
    }
}

fn app_config_path() -> Option<(PathBuf, PathBuf)> {
    if let Some(proj_dir) = ProjectDirs::from("", "", "zenoh-hammer") {
        let acd = proj_dir.config_dir().to_path_buf();
        let mut acp = acd.clone();
        acp.push("last_opened_file");
        Some((acd, acp))
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
