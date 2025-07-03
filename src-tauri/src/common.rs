use chrono::{DateTime, Local};
use std::process::Command;
use sysinfo::{Pid, System};
use url::Url;

pub fn get_pid_hosting_at(url: Url) -> Option<u32> {
    let port = url.port()?;
    println!("Port: {}", port);

    #[cfg(all(target_os = "linux", target_os = "macos"))]
    {
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
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("netstat").args(["-ano"]).output().ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            if line.contains(&format!(":{}", port)) {
                if let Some(pid_str) = line.split_whitespace().last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        return Some(pid);
                    }
                }
            }
        }
        None
    }
}

pub fn get_process_start_time(pid: u32) -> Option<String> {
    let mut sys = System::new_all();

    // First we update all information of our `System` struct.
    sys.refresh_all();

    if let Some(process) = sys.process(Pid::from_u32(pid)) {
        let date: String = DateTime::from_timestamp(process.start_time() as i64, 0)
            .unwrap()
            .format("%d/%m/%Y %H:%M").to_string();
        println!("{}", date);
        return Some(date);
    }

    None
}
