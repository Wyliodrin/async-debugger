use std::process::Command;
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
