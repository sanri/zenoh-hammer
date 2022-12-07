use async_std::task;
use std::thread;
use zenoh::prelude::*;


pub fn start_async() {
    thread::spawn(move || task::block_on(loop_zenoh()));
}

async fn loop_zenoh() {

}
