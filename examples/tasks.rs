use std::io;
use std::sync::Arc;
use tokio::sync::Barrier;
use tokio::time::{sleep, Duration, Instant};

/// Track this app at localhost:7777
/// 
/// Waits for user input to start, then spawns three tasks that simulate work,
/// wait on a common barrier, and then proceed together. After another pause,
/// it optionally burns CPU cycles if `burn_cpu_for` is enabled.
#[tokio::main]
async fn main() {
    console_subscriber::ConsoleLayer::builder()
        .with_default_env()
        .server_addr(([127, 0, 0, 1], 7777))
        .init();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    {
        /// A barrier that blocks until three tasks have reached it.
        let barrier = Arc::new(Barrier::new(3));
        
        for i in 1..=3 {
            let b = barrier.clone();
            tokio::spawn(async move {
                println!("Task {i} started and is doing some work...");
                sleep(Duration::from_millis(i * 500)).await;

                println!("Task {i} waiting at the barrier...");
                b.wait().await;

                println!("Task {i} passed the barrier!");
            });
        }
        
        sleep(Duration::from_secs(3)).await;
    }

    println!("se foloseste procesorul");
    
    io::stdin().read_line(&mut input).unwrap();
}

/// Busy-waits to consume CPU time for the specified duration.
///
/// Uses a wrapping arithmetic loop to prevent compiler optimizations.
///
/// # Arguments
///
/// * `seconds` - Number of seconds to burn CPU cycles.
fn burn_cpu_for(seconds: u64) {
    let start = Instant::now();
    let mut x = 0u64;
    
    while start.elapsed() < Duration::from_secs(seconds) {
        x = x.wrapping_add(1);
        x = x.wrapping_mul(2);
    }
    
    println!("Finished CPU burn, final x = {}", x);
}
