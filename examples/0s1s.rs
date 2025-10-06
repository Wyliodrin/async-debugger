use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

/// Track this app at localhost:6669
///
/// This app spawns three asynchronous tasks:
/// 1. A producer that sends 1024-byte zero-filled chunks every 500 ms.
/// 2. A producer that sends 1024-byte one-filled chunks every 700 ms.
/// 3. A consumer that concurrently reads from both channels and logs incoming data.
#[tokio::main]
async fn main() {
    console_subscriber::init();

    /// Transmitter for zero-filled data chunks.
    let (zero_tx, mut zero_rx) = mpsc::channel::<Vec<u8>>(10);
    /// Transmitter for one-filled data chunks.
    let (one_tx, mut one_rx) = mpsc::channel::<Vec<u8>>(10);

    /// Producer task that generates and sends chunks of zeros.
    let zero_handle = tokio::spawn(async move {
        loop {
            // Create a 1024-byte chunk of zeros
            let chunk = vec![0u8; 1024];
            // Attempt to send; exit loop on failure (receiver dropped)
            if zero_tx.send(chunk).await.is_err() {
                break;
            }
            println!("Sent 1 chunk of 0s");
            // Wait 500 milliseconds before next send
            sleep(Duration::from_millis(500)).await;
        }
    });

    /// Producer task that generates and sends chunks of ones.
    let one_handle = tokio::spawn(async move {
        loop {
            let chunk = vec![1u8; 1024];
            if one_tx.send(chunk).await.is_err() {
                break;
            }
            println!("Sent 1 chunk of 1s");
            sleep(Duration::from_millis(700)).await;
        }
    });

    /// Consumer task that listens to both zero and one channels and processes incoming data.
    let consumer_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle messages from the zero channel
                maybe0 = zero_rx.recv() => {
                    match maybe0 {
                        Some(data) => {
                            println!("Received {} bytes of zeros", data.len());
                        }
                        None => {
                            println!("Zero channel closed");
                            break;
                        }
                    }
                }

                // Handle messages from the one channel
                maybe1 = one_rx.recv() => {
                    match maybe1 {
                        Some(data) => {
                            println!("Received {} bytes of ones", data.len());
                        }
                        None => {
                            println!("One channel closed");
                            break;
                        }
                    }
                }
            }
        }
    });

    // Await all three tasks before exiting
    let _ = tokio::join!(zero_handle, one_handle, consumer_handle);
}
