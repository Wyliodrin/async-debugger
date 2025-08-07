use chrono::DateTime;
use serde::Serialize;
use std::fmt::Debug;
use std::sync::Arc;
use std::{collections::HashMap, process::Command};
use sysinfo::{Pid, System};
use url::Url;

use crate::domain::has_app_name::HasAppName;
use crate::infra::guard::WriteableDataBaseGuard;

/// Returns the PID of the process listening on the given URL’s port, if any.
///
/// # Arguments
///
/// * `url` – A `Url` instance pointing to the target host and port.
///
/// # Returns
///
/// * `Some(u32)` containing the PID of the process bound to that port, or
/// * `None` if the URL has no defined port or if no hosting process is found.
///
/// # Platform-specific behavior
///
/// On Linux and macOS, this spawns `lsof -ti :<port>` and parses its output.
/// On Windows, this spawns `netstat -ano` and searches for the port in each line.
pub fn get_pid_hosting_at(url: Url) -> Option<u32> {
    // Extract the port number from the URL; return None if absent.
    let port = url.port()?;
    println!("Port: {}", port);

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Use lsof to find processes listening on that port.
        let output = Command::new("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output()
            .ok()?;

        // If no output, no process found
        if output.stdout.is_empty() {
            return None;
        }

        // Parse the PID out of the stdout
        let output_str = String::from_utf8_lossy(&output.stdout);
                // In case the tracked app is restarted, lsof prints two PIDS and the correct PID is typically the last whitespace-separated token.
                if let Some(pid_str) = output_str.lines().last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        println!("{:?}", pid);
                        return Some(pid);
                    }
                }
        None
    }

    #[cfg(target_os = "windows")]
    {
        // Use netstat to list all TCP/UDP connections with PIDs.
        let output = Command::new("netstat").args(["-ano"]).output().ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Look for lines containing “:<port>”
        for line in stdout.lines() {
            if line.contains(&format!(":{}", port)) {
                // The PID is typically the last whitespace-separated token.
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

/// Retrieves the process start time for a given PID, formatted as a human-readable string.
///
/// # Arguments
///
/// * `pid` – Process identifier (u32).
///
/// # Returns
///
/// * `Some(String)` containing the start time in the format `DD/MM/YYYY HH:MM:SS`, or
/// * `None` if no process with that PID exists.
pub fn get_process_start_time(pid: u32) -> Option<String> {
    // Initialize a sysinfo System instance and gather all data.
    let mut sys = System::new_all();
    sys.refresh_all();

    // Look up the process by PID.
    if let Some(process) = sys.process(Pid::from_u32(pid)) {
        // Convert the process start time (seconds since epoch) into a formatted string.
        let date: String = DateTime::from_timestamp(process.start_time() as i64, 0)
            .unwrap()
            .format("%d/%m/%Y %H:%M:%S")
            .to_string();
        return Some(date);
    }

    // PID not found in the system
    None
}

/// Renames keys in the given database guard by replacing the `old_title` prefix
/// in keys with the `new_title` prefix. The function iterates over all keys,
/// collects the keys that start with `old_title.`, and renames them accordingly.
///
/// # Arguments
///
/// * `guard` - A mutable reference to a writable database guard holding a `HashMap`
///             where keys are strings and values are of generic type `T`.
/// * `new_title` - The new title string to replace the old title prefix in keys.
/// * `old_title` - The old title string prefix to be replaced in keys.
///
/// # Type Parameters
///
/// * `T` - The type of the values in the hashmap. Must implement `Serialize` and `Debug`.
pub fn rename_database_keys<T: Serialize + Debug>(
    guard: &mut WriteableDataBaseGuard<'_, HashMap<String, T>>,
    new_title: String,
    old_title: String,
) {
    let mut changes = Vec::new();
    for key in guard.keys() {
        if let Some(rest) = key.strip_prefix(&format!("{}.", old_title)) {
            let new_key = format!("{}.{}", new_title, rest);
            changes.push((key.clone(), new_key));
        }
    }

    for (old, new) in changes {
        if let Some(val) = guard.remove(&old) {
            guard.insert(new, val);
        }
    }
}

/// Renames keys in the given database guard by replacing the `old_title` prefix
/// in keys with the `new_title` prefix. In addition, it updates the `app_name`
/// attribute of the value associated with each renamed key.
///
/// This function works on database guards containing `HashMap<String, Arc<T>>`,
/// where `T` must implement `Serialize`, `Debug`, `HasAppName`, and `Clone`.
///
/// # Arguments
///
/// * `guard` - A mutable reference to a writable database guard holding a `HashMap`
///             where keys are strings and values are `Arc` wrapped generic type `T`.
/// * `new_title` - The new title string to replace the old title prefix in keys.
/// * `old_title` - The old title string prefix to be replaced in keys.
///
/// # Type Parameters
///
/// * `T` - The type of the values inside the Arc in the hashmap. Must implement
///         `Serialize`, `Debug`, `HasAppName` (a trait providing `set_app_name`), and `Clone`.
pub fn rename_database_keys_and_app_name<T: Serialize + Debug + HasAppName + Clone>(
    guard: &mut WriteableDataBaseGuard<'_, HashMap<String, Arc<T>>>,
    new_title: String,
    old_title: String,
) {
    let mut changes = Vec::new();
    for key in guard.keys() {
        if let Some(rest) = key.strip_prefix(&format!("{}.", old_title)) {
            let new_key = format!("{}.{}", new_title, rest);
            changes.push((key.clone(), new_key));
        }
    }

    for (old, new) in changes {
        if let Some(mut val_arc) = guard.remove(&old) {
            let val = Arc::make_mut(&mut val_arc);
            val.set_app_name(new_title.clone());
            guard.insert(new, Arc::new(val.clone()));
        }
    }
}
