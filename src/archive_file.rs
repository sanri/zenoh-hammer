use serde::{Deserialize, Serialize};
use serde_json;
use std::{fs, path::Path};

use crate::{
    page_get::ArchivePageGet, page_put::ArchivePagePut, page_session::ArchivePageSession,
    page_sub::ArchivePageSub,
};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ArchiveApp {
    pub page_session: ArchivePageSession,
    pub page_sub: ArchivePageSub,
    pub page_put: ArchivePagePut,
    pub page_get: ArchivePageGet,
}

impl ArchiveApp {
    pub fn load(path: &Path) -> Result<ArchiveApp, String> {
        let body = match fs::read(path) {
            Ok(o) => o,
            Err(e) => {
                return Err(e.to_string());
            }
        };

        let body_str = match String::from_utf8(body) {
            Ok(o) => o,
            Err(e) => {
                return Err(e.to_string());
            }
        };

        Self::from_str(body_str.as_str())
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let s = self.to_string();
        let body = s.as_bytes();
        match fs::write(path, body) {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn from_str(s: &str) -> Result<ArchiveApp, String> {
        match serde_json::from_str::<ArchiveApp>(s) {
            Ok(o) => Ok(o),
            Err(e) => Err(e.to_string()),
        }
    }

    fn to_string(&self) -> String {
        let json: serde_json::Value = serde_json::to_value(&self).unwrap();
        format!("{:#}", json)
    }
}

#[test]
fn app_file_to_string() {
    let app_file = ArchiveApp::default();
    let s = app_file.to_string();
    println!("{}", s);
}

#[test]
fn app_file_write() {
    use std::path::PathBuf;
    use std::str::FromStr;

    let app_file = ArchiveApp::default();
    match app_file.write(PathBuf::from_str("target/app_file.json").unwrap().as_path()) {
        Ok(_) => {
            println!("write ok");
        }
        Err(e) => {
            println!("{}", e);
        }
    }
}
