#[path = "../src/data_viewer.rs"]
mod data_viewer;
#[path = "../src/hex_viewer.rs"]
mod hex_viewer;
#[path = "../src/sample_viewer.rs"]
mod sample_viewer;
#[path = "../src/zenoh_data.rs"]
mod zenoh_data;

use eframe::{
    egui::{CentralPanel, ComboBox, Context, ScrollArea, Ui},
    AppCreator, Frame, HardwareAcceleration, NativeOptions,
};
use env_logger::Env;
use std::time::{SystemTime, UNIX_EPOCH};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};
use uhlc::{Timestamp, ID, NTP64};
use zenoh::{
    bytes::{Encoding, ZBytes},
    sample::{SampleKind, SourceInfo},
};

use crate::{
    sample_viewer::{SampleInfo, SampleViewer},
    zenoh_data::{BytesType, ZCongestionControl, ZPriority, ZReliability},
};

#[derive(Eq, PartialEq, Copy, Clone, AsRefStr, EnumIter)]
#[strum(serialize_all = "snake_case")]
enum Page {
    Bytes,
    String,
    Json,
    Json5,
    Png,
}

struct AppHexViewer {
    selected_page: Page,
    viewer: SampleViewer,
}

impl Default for AppHexViewer {
    fn default() -> Self {
        let selected_page = Page::Bytes;
        let (base_info, data) = example_data(selected_page);
        let viewer = SampleViewer::new(base_info, data);
        AppHexViewer {
            selected_page,
            viewer,
        }
    }
}

impl AppHexViewer {
    fn show(&mut self, ui: &mut Ui) {
        self.show_select(ui);

        ui.separator();

        ScrollArea::both().show(ui, |ui| {
            self.viewer.show(ui);
        });
    }

    fn show_select(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Page");
            ComboBox::new("show_sample_viewer_combobox", "")
                .selected_text(self.selected_page.as_ref())
                .show_ui(ui, |ui| {
                    for selected in Page::iter() {
                        let r = ui.selectable_value(
                            &mut self.selected_page,
                            selected,
                            selected.as_ref(),
                        );
                        if r.changed() {
                            let (base_info, data) = example_data(selected);
                            self.viewer = SampleViewer::new(base_info, data);
                        }
                    }
                });
        });
    }
}

impl eframe::App for AppHexViewer {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| self.show(ui));
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("show_sample_viewer=info"))
        .init();

    let options = NativeOptions {
        hardware_acceleration: HardwareAcceleration::Required,
        ..NativeOptions::default()
    };
    let app = AppHexViewer::default();
    let create: AppCreator = Box::new(|_cc| Ok(Box::new(app)));
    let _ = eframe::run_native("SampleViewer", options, create);
}

fn example_base_info() -> SampleInfo {
    let key = "test/a/b/c/1".to_string();
    let encoding = Encoding::default();
    let kind = SampleKind::Put;
    let time = NTP64::from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap());
    let timestamp = Some(Timestamp::new(time, ID::rand()));
    let congestion_control = ZCongestionControl::Block;
    let priority = ZPriority::RealTime;
    let reliability = ZReliability::Reliable;
    let express = true;
    let source_info = SourceInfo::new(None, Some(123));
    let attachment = b"a=1".to_vec();
    let bytes_type = BytesType::Raw;

    SampleInfo {
        key,
        encoding,
        kind,
        timestamp,
        congestion_control,
        priority,
        reliability,
        express,
        source_info,
        attachment,
        bytes_type,
    }
}

fn example_data(page: Page) -> (SampleInfo, Vec<u8>) {
    let mut base_info = example_base_info();
    let z_bytes = match page {
        Page::Bytes => {
            base_info.encoding = Encoding::ZENOH_BYTES;
            ZBytes::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
        }
        Page::String => {
            base_info.encoding = Encoding::ZENOH_STRING;
            ZBytes::from("hello world")
        }
        Page::Json => {
            base_info.encoding = Encoding::TEXT_JSON;
            let s = data_json();
            ZBytes::from(s)
        }
        Page::Json5 => {
            base_info.encoding = Encoding::TEXT_JSON5;
            let s = data_json5();
            ZBytes::from(s)
        }
        Page::Png => {
            base_info.encoding = Encoding::IMAGE_PNG;
            let d = data_png();
            ZBytes::from(d)
        }
    };

    let buf = z_bytes.to_bytes().to_vec();

    (base_info, buf)
}

fn data_json() -> String {
    let s = r#"
{
  "page_get": {
    "gets": [
      { "consolidation": "auto", "key": "demo/test", "locality": "any", "name": "demo", "target": "best_matching", "timeout": 10000, "value": { "type": "Empty" } }
    ]
  },
  "page_put": {
    "puts": [
      { "congestion_control": "block", "key": "demo/robot", "name": "demo", "priority": "real_time", "value": { "data": "{\n\t\"axis1\": 90,\n\t\"axis2\": -60,\n\t\"axis3\": 90,\n\t\"axis4\": -30,\n\t\"axis5\": -90,\n\t\"axis6\": 20,\n\t\"gripper\": 20\n}", "type": "TextJson" } }
    ]
  },
  "page_sub": {
    "subscribers": [
      { "key_expr": "demo/**", "name": "demo" }
    ]
  }
}"#;

    s.to_string()
}

fn data_json5() -> String {
    let s = r#"
    {
  "page_get": {
    "gets": [
      {
        "consolidation": "auto",
        "key": "demo/test",
        "locality": "any",
        "name": "demo",
        "target": "best_matching",
        "timeout": 10000,
        "value": {
          "type": "Empty"
        }
      }
    ]
  },
  "page_put": {
    "puts": [
      {
        "congestion_control": "block",
        "key": "satellite_slave/robot_system/w/request_stream",
        "name": "move_standby",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"move_standby\", \"id\": \"092340920\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_slave/robot_system/w/request_stream",
        "name": "shooting_position",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"move_shooting_position\", \"id\": \"092340920\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_slave/robot_system/w/request_stream",
        "name": "capture",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"capture\", \"id\": \"092340920\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_slave/robot_system/w/request_stream",
        "name": "free",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"free\", \"id\": \"092340920\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_slave/robot_left/w/request_stream",
        "name": "robot ",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"free\", \"id\": \"092340920\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_target/motion_control/w/request_stream",
        "name": "target move p0",
        "priority": "real_time",
        "value": {
          "data": "{ \n    \"cmd\": \"ptp\", \"id\": \"move to p0\", \n    \"target_pos\": { \n\t\t\"x\": 2758.0, \n\t\t\"y\": 1000.0, \n\t\t\"alpha\": 0.0\n\t} \n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_slave/motion_control/w/request_stream",
        "name": "slave move p0",
        "priority": "real_time",
        "value": {
          "data": "{ \n    \"cmd\": \"ptp\", \"id\": \"move to p0\", \n    \"target_pos\": { \n\t\t\"x\": 2754, \n\t\t\"y\": 6100.0, \n\t\t\"alpha\": 0.0 \n\t} \n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_target/motion_control/w/request_stream",
        "name": "target move p1",
        "priority": "real_time",
        "value": {
          "data": "{ \n    \"cmd\": \"ptp\", \"id\": \"move to p0\", \n    \"target_pos\": { \n\t\t\"x\": 2800.0, \n\t\t\"y\": 2723.0, \n\t\t\"alpha\": 90.0\n\t} \n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_slave/motion_control/w/request_stream",
        "name": "slave move p1",
        "priority": "real_time",
        "value": {
          "data": "{ \n    \"cmd\": \"ptp\", \"id\": \"move to p0\", \n    \"target_pos\": { \n\t\t\"x\": 2761.0, \n\t\t\"y\": 4537.0, \n\t\t\"alpha\": -90.0\n\t} \n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "begin_write",
        "priority": "real_time",
        "value": {
          "data": "{ \n\t\"cmd\": \"begin_write\", \n\t\"text\": \"高压放气测试\" \n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "end_write",
        "priority": "real_time",
        "value": {
          "data": "{ \"cmd\": \"end_write\" }",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "play_history",
        "priority": "real_time",
        "value": {
          "data": "{ \n    \"cmd\": \"change_pub_data\", \n    \"target_state\": \"history\", \n    \"id\": 10, \n    \"playing\": true\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "play_realtime",
        "priority": "real_time",
        "value": {
          "data": "{ \n    \"cmd\": \"change_pub_data\", \n    \"target_state\": \"real_time\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "play_not",
        "priority": "real_time",
        "value": {
          "data": "{ \n    \"cmd\": \"change_pub_data\", \n    \"target_state\": \"not\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "play",
        "priority": "real_time",
        "value": {
          "data": "{\n   \"cmd\": \"play_control\", \n\t\"action\": \"play\", \n\t\"time\": 12.0\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "goto_play",
        "priority": "real_time",
        "value": {
          "data": "{\n   \"cmd\": \"play_control\", \n\t\"action\": \"goto_play\", \n\t\"time\": 10.0\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "pause",
        "priority": "real_time",
        "value": {
          "data": "{\n   \"cmd\": \"play_control\", \n\t\"action\": \"pause\", \n\t\"time\": 12.0\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "goto_pause",
        "priority": "real_time",
        "value": {
          "data": "{\n   \"cmd\": \"play_control\", \n\t\"action\": \"goto_pause\", \n\t\"time\": 140.0\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "storage/w/request_stream",
        "name": "get_logs",
        "priority": "real_time",
        "value": {
          "data": "{ \"cmd\": \"get_logs\" }",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_change_state dormant",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"change_state\", \n\t\"id\": \"092340920\", \n\t\"target_state\": \"dormant\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_change_state running",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"change_state\", \n\t\"id\": \"092340920\", \n\t\"target_state\": \"running\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_slave go_home",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"satellite_slave\", \"id\": \"092340920\", \"action\": \"go_home\",\n    \"target_pos\": {\n        \"x\": 0.0, \"y\": 0.0, \"alpha\": 0.0\n    }\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_slave dormant",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"satellite_slave\", \"id\": \"092340920\", \"action\": \"dormant\",\n    \"target_pos\": {\n        \"x\": 0.0, \"y\": 0.0, \"alpha\": 0.0\n    }\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_slave tracking",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"satellite_slave\", \"id\": \"092340920\", \"action\": \"tracking\",\n    \"target_pos\": {\n        \"x\": 0.0, \"y\": 0.0, \"alpha\": 0.0\n    }\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_slave cease_tracking",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"satellite_slave\", \"id\": \"092340920\", \"action\": \"cease_tracking\",\n    \"target_pos\": {\n        \"x\": 0.0, \"y\": 0.0, \"alpha\": 0.0\n    }\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_target go_home",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"satellite_target\", \"id\": \"092340920\", \"action\": \"go_home\",\n    \"target_pos\": {\n        \"x\": 0.0, \"y\": 0.0, \"alpha\": 0.0\n    },\n\t\"path_no\": 1\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_target dormant",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"satellite_target\", \"id\": \"092340920\", \"action\": \"dormant\",\n    \"target_pos\": {\n        \"x\": 0.0, \"y\": 0.0, \"alpha\": 0.0\n    },\n\t\"path_no\": 1\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_target circulation_path_motion",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"satellite_target\", \"id\": \"092340920\", \"action\": \"circulation_path_motion\",\n    \"target_pos\": {\n        \"x\": 0.0, \"y\": 0.0, \"alpha\": 0.0\n    },\n\t\"path_no\": 1\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_robot capture",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"robot_system\", \"id\": \"092340920\", \"action\": \"capture\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "master_control/w/request_stream",
        "name": "ma_robot free",
        "priority": "real_time",
        "value": {
          "data": "{\n    \"cmd\": \"robot_system\", \"id\": \"092340920\", \"action\": \"free\"\n}",
          "type": "AppJson"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_target/flywheel/w/command",
        "name": "flywheel target command",
        "priority": "real_time",
        "value": {
          "data": 3,
          "type": "AppInteger"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_target/flywheel/w/target_value",
        "name": "flywheel target target_value",
        "priority": "real_time",
        "value": {
          "data": 10,
          "type": "AppInteger"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_slave/flywheel/w/command",
        "name": "flywheel slave command",
        "priority": "real_time",
        "value": {
          "data": 0,
          "type": "AppInteger"
        }
      },
      {
        "congestion_control": "block",
        "key": "satellite_slave/flywheel/w/target_value",
        "name": "flywheel slave target_value",
        "priority": "real_time",
        "value": {
          "data": 0,
          "type": "AppInteger"
        }
      }
    ]
  },
  "page_sub": {
    "subscribers": [
      {
        "key_expr": "satellite_target/**",
        "name": "target"
      },
      {
        "key_expr": "satellite_slave/**",
        "name": "slave"
      },
      {
        "key_expr": "satellite_slave/robot_system/**",
        "name": "robot_system"
      },
      {
        "key_expr": "motion_capture/**",
        "name": "motion_capture"
      },
      {
        "key_expr": "storage/**",
        "name": "storage"
      },
      {
        "key_expr": "digital_twin/**",
        "name": "digital_twin"
      },
      {
        "key_expr": "master_control/**",
        "name": "master_control"
      }
    ]
  }
} "#;
    s.to_string()
}

fn data_png() -> Vec<u8> {
    let d = include_bytes!("../media/hammer.png");
    Vec::from(d)
}
