use std::io;
use std::sync::Arc;
use tokio::sync::{Barrier, RwLock};
use tokio::time::{Duration, Instant, sleep};

#[tokio::main]
async fn main() {
    console_subscriber::ConsoleLayer::builder()
        .with_default_env()
        .server_addr(([127, 0, 0, 1], 7777))
        .init();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    println!("press something");
    {
        let barrier = Arc::new(Barrier::new(3));
        let sum = Arc::new(RwLock::new(0));

        for i in 1..=3 {
            let b = barrier.clone();
            let s = sum.clone();
            tokio::spawn(async move {
                println!("Task {i} started and is doing some work...");
                sleep(Duration::from_millis(i * 500)).await;
                {
                    let mut mut_sum = s.write().await;
                    *mut_sum += i;
                }

                println!("Task {i} waiting at the barrier...");
                b.wait().await;

                println!("Task {i} passed the barrier!");
            });
        }

        sleep(Duration::from_secs(3)).await;
    }
    println!("stressing cpu");
    //burn_cpu_for(20);
    println!("press something to exit");
    io::stdin().read_line(&mut input).unwrap();
}

fn burn_cpu_for(seconds: u64) {
    let start = Instant::now();
    let mut x = 0u64;

    while start.elapsed() < Duration::from_secs(seconds) {
        x = x.wrapping_add(1);
        x = x.wrapping_mul(2);
    }

    println!("Finished CPU stress, final x = {}", x);
}
