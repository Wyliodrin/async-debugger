// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use env_logger as _;
use log::info;

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Starting");
    tokio_display_lib::run().await
}
