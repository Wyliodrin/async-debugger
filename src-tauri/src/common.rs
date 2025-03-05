use crate::error::Error as TraceError;
use chrono::{DateTime, Local, TimeZone};
use log::error;
use std::process::Command;
use sysinfo::{Pid, System};
use url::Url;

pub fn get_pid_hosting_at(url: Url) -> Option<u32> {
    let port = url.port()?;
    println!("Port: {}", port);

    let output = Command::new("lsof")
        .args(["-ti", &format!(":{}", port)])
        .output()
        .ok()?;

    if output.stdout.is_empty() {
        // No process hosting on the specified url has been found
        return None;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    output_str.trim().parse().ok()
}

pub fn get_process_start_time(pid: u32) -> Option<DateTime<Local>> {
    let mut sys = System::new_all();

    // First we update all information of our `System` struct.
    sys.refresh_all();

    if let Some(process) = sys.process(Pid::from_u32(pid)) {
        let date: DateTime<Local> = DateTime::from_timestamp(process.start_time() as i64, 0)
            .unwrap()
            .with_timezone(&Local);

        return Some(date);
    }

    None
}
