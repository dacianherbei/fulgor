//! Basic usage example for AsyncEventReceiver
//!
//! This example demonstrates how to use the AsyncEventReceiver with
//! different event types, showing the template flexibility.

use fulgor::renderer::{RendererEvent, RendererKind};
use async_channel;
use std::time::Duration;
use fulgor::AsyncEventReceiver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fulgor AsyncEventReceiver Example");
    println!("=================================");

    // Example 1: Using with the built-in RendererEvent type
    await_renderer_events_example().await?;

    // Example 2: Using with a custom event type
    await_custom_events_example().await?;

    println!("All examples completed successfully!");
    Ok(())
}

/// Demonstrates usage with the built-in RendererEvent type
async fn await_renderer_events_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Renderer Events Example ---");

    // Create an unbounded channel for RendererEvent
    let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    let event_receiver = AsyncEventReceiver::new(receiver);

    // Send some example events
    let events = vec![
        RendererEvent::ViewportResized { width: 1920, height: 1080 },
        RendererEvent::SplatDataUpdated { splat_count: 150000 },
        RendererEvent::FrameRendered { renderer_kind: RendererKind::CpuReference, frame_number: 1, frame_time_microseconds: 0, render_time_ns: 1667 },
        RendererEvent::FrameRendered { renderer_kind: RendererKind::CpuReference, frame_number: 2, frame_time_microseconds: 0, render_time_ns: 1532 },
    ];

    // Send events in a separate task
    let sender_task = tokio::spawn(async move {
        for event in events {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Err(e) = sender.send(event).await {
                eprintln!("Failed to send event: {}", e);
                break;
            }
        }
        drop(sender); // Close the channel
    });

    // Receive and process events
    let mut frame_count = 0;
    loop {
        // Check channel status
        println!("Channel status - Length: {}, Empty: {}, Closed: {}",
                 event_receiver.len(),
                 event_receiver.is_empty(),
                 event_receiver.is_closed());

        // Try to receive an event
        match event_receiver.recv().await {
            Ok(event) => {
                match event {
                    RendererEvent::FrameRendered { frame_number, render_time_ns, .. } => {
                        frame_count += 1;
                        println!("  Processed frame {} in {:.2}ms", frame_number, render_time_ns);
                    }
                    RendererEvent::ViewportResized { width, height } => {
                        println!("  Viewport resized to {}x{}", width, height);
                    }
                    RendererEvent::SplatDataUpdated { splat_count } => {
                        println!("  Splat data updated with {} splats", splat_count);
                    }
                    RendererEvent::Error { message, .. } => {
                        println!("  Render error: {}", message);
                    }
                    RendererEvent::ShutdownRequested => {
                        println!("  Shutdown requested");
                        break;
                    }
                    _ => {} // TODO: implement handling for the oder events
                }
            }
            Err(e) => {
                println!("  Channel closed: {}", e);
                break;
            }
        }
    }

    sender_task.await?;
    println!("Processed {} frames total", frame_count);
    Ok(())
}

/// Demonstrates usage with a custom event type
async fn await_custom_events_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Custom Events Example ---");

    #[derive(Debug, Clone)]
    enum CustomEvent {
        UserInput { key: String },
        NetworkMessage { data: Vec<u8> },
        TimerExpired { id: u32 },
    }

    // Create channel for custom events
    let (sender, receiver) = async_channel::bounded::<CustomEvent>(10);
    let event_receiver = AsyncEventReceiver::new(receiver);

    // Send some custom events
    tokio::spawn(async move {
        let custom_events = vec![
            CustomEvent::UserInput { key: "Space".to_string() },
            CustomEvent::NetworkMessage { data: vec![1, 2, 3, 4] },
            CustomEvent::TimerExpired { id: 42 },
        ];

        for event in custom_events {
            if let Err(e) = sender.send(event).await {
                eprintln!("Failed to send custom event: {}", e);
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    // Process custom events
    for _ in 0..3 {
        // First try non-blocking receive
        match event_receiver.try_recv() {
            Ok(event) => {
                println!("  Immediately received: {:?}", event);
                continue;
            }
            Err(async_channel::TryRecvError::Empty) => {
                println!("  No event immediately available, waiting...");
            }
            Err(async_channel::TryRecvError::Closed) => {
                println!("  Channel closed");
                break;
            }
        }

        // If no immediate event, wait for one
        match event_receiver.recv().await {
            Ok(event) => {
                println!("  Received after waiting: {:?}", event);
            }
            Err(e) => {
                println!("  Error receiving: {}", e);
                break;
            }
        }
    }

    Ok(())
}