
default:
    @just --list --unsorted

build_debug:
    cargo build -package zenoh-hammer

build_release:
    cargo build --release --package zenoh-hammer

build_lto:
    cargo build --profile release-lto --package zenoh-hammer

run_hex_viewer:
    cargo run --package zenoh-hammer --example show_hex_viewer

run_payload_editor:
    cargo run --package zenoh-hammer --example show_payload_editor

run_sample_viewer:
    cargo run --package zenoh-hammer --example show_sample_viewer
