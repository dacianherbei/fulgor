//! Basic usage example for AsyncEventReceiver
//!
//! This example demonstrates how to use the AsyncEventReceiver with
//! different event types and configurations, showing the template flexibility.

use fulgor::renderer::{RendererEvent};
use fulgor::{AsyncEventReceiver, AsyncChannelConfig};
use async_channel;
use futures::StreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fulgor AsyncEventReceiver Example");
    println!("=================================");

    // Example 1: Using with the built-in RendererEvent type and default f64 precision
    await_renderer_events_example().await?;

    // Example 2: Using with a custom event type and f32 precision
    await_custom_events_example().await?;

    // Example 3: Demonstrating Stream interface
    await_stream_interface_example().await?;

    // Example 4: Configuration showcase
    await_configuration_showcase().await?;

    println!("All examples completed successfully!");
    Ok(())
}

/// Demonstrates usage with the built-in RendererEvent type and high-throughput configuration
async fn await_renderer_events_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Renderer Events Example (High-Throughput Config) ---");

    // Create a high-throughput configuration optimized for rendering workloads
    let config = AsyncChannelConfig{ maximum_buffer_size: 0, send_timeout: Some(Duration::from_millis(5)), enable_backpressure: false, statistics_interval: Default::default() };
    println!("Using config - Buffer: {}, Send Timeout: {:?}, Backpressure: {}, Statistics Interval: {:?}",
             config.maximum_buffer_size,
             config.send_timeout.unwrap(),
             config.enable_backpressure,
             config.statistics_interval);

    // Create an unbounded channel for RendererEvent
    let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    let event_receiver = AsyncEventReceiver::new(receiver, config);

    // Send some example events
    let events = vec![
        RendererEvent::ViewportResized { renderer_id: 1, width: 1920, height: 1080 },
        RendererEvent::SplatDataUpdated { renderer_id: 1, splat_count: 150000 },
        RendererEvent::FrameRendered {
            renderer_id: 1,
            frame_number: 1,
            frame_time_microseconds: 0,
            render_time_ns: 1667
        },
        RendererEvent::FrameRendered {
            renderer_id: 1,
            frame_number: 2,
            frame_time_microseconds: 0,
            render_time_ns: 1532
        },
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
        match event_receiver.receive_event().await {
            Ok(event) => {
                match event {
                    RendererEvent::FrameRendered { frame_number, render_time_ns, .. } => {
                        frame_count += 1;
                        println!("  Processed frame {} in {:.2}µs", frame_number, render_time_ns);
                    }
                    RendererEvent::ViewportResized { renderer_id: 1, width, height } => {
                        println!("  Viewport resized to {}x{}", width, height);
                    }
                    RendererEvent::SplatDataUpdated { renderer_id: 1, splat_count } => {
                        println!("  Splat data updated with {} splats", splat_count);
                    }
                    RendererEvent::RendererError{ message, .. } => {
                        println!("  Render error: {}", message);
                    }
                    RendererEvent::Shutdown{ .. } => {
                        println!("  Shutdown requested");
                        break;
                    }
                    _ => {
                        println!("  Other event: {:?}", event);
                    }
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
    println!("Total events received: {}", event_receiver.received_events_count());
    Ok(())
}

/// Demonstrates usage with a custom event type and f32 precision for low-latency requirements
async fn await_custom_events_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Custom Events Example (Low-Latency f32 Config) ---");

    #[derive(Debug, Clone)]
    enum CustomEvent {
        UserInput { key: String },
        NetworkMessage { data: Vec<u8> },
        TimerExpired { id: u32 },
    }

    // Create a low-latency configuration with f32 precision for real-time applications
    let config = AsyncChannelConfig::low_latency(0.001f32);
    println!("Using f32 precision config - Buffer: {}, Timeout: {:?}",
             config.maximum_buffer_size,
             config.timeout());

    // Create channel for custom events
    let (sender, receiver) = async_channel::bounded::<CustomEvent>(10);
    let event_receiver = AsyncEventReceiver::new(receiver, config);

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
        match event_receiver.try_receive_event() {
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
        match event_receiver.receive_event().await {
            Ok(event) => {
                println!("  Received after waiting: {:?}", event);
            }
            Err(e) => {
                println!("  Error receiving: {}", e);
                break;
            }
        }
    }

    println!("Custom events received: {}", event_receiver.received_events_count());
    println!("Configuration precision threshold: {}", event_receiver.configuration().precision());
    Ok(())
}

/// Demonstrates the Stream interface functionality
async fn await_stream_interface_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Stream Interface Example ---");

    #[derive(Debug, Clone, PartialEq)]
    enum StreamEvent {
        Data(i32),
        End,
    }

    // Use bounded configuration with backpressure for stream processing
    let config = AsyncChannelConfig::bounded_with_backpressure(5);
    let (sender, receiver) = async_channel::bounded::<StreamEvent>(10);
    let mut event_receiver = AsyncEventReceiver::new(receiver, config);

    // Send a sequence of events
    tokio::spawn(async move {
        for i in 1..=10 {
            sender.send(StreamEvent::Data(i)).await.unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        sender.send(StreamEvent::End).await.unwrap();
    });

    println!("Processing events as a Stream:");

    // Use the Stream interface to process events
    let mut count = 0;
    while let Some(event) = event_receiver.next().await {
        match event {
            StreamEvent::Data(value) => {
                count += 1;
                println!("  Stream event {}: Data({})", count, value);
            }
            StreamEvent::End => {
                println!("  Stream end signal received");
                break;
            }
        }

        // Demonstrate that we can still access receiver properties
        if count % 3 == 0 {
            println!("    - Queue length: {}, Events processed: {}",
                     event_receiver.len(),
                     event_receiver.received_events_count());
        }
    }

    println!("Stream processing complete. Total events: {}", event_receiver.received_events_count());
    Ok(())
}

/// Showcases different configuration options and their effects
async fn await_configuration_showcase() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Configuration Showcase ---");

    // Example with different configurations
    let configs = vec![
        ("Default f64", AsyncChannelConfig::default()),
        ("Unbounded", AsyncChannelConfig::unbounded()),
        ("Bounded(100)", AsyncChannelConfig::bounded(100))
    ];

    for (name, config) in configs {
        println!("\n{} Configuration:", name);
        println!("  - Buffer size: {}",
                 if config.is_unbounded() { "Unlimited".to_string() } else { config.maximum_buffer_size.to_string() });
        println!("  - Backpressure: {}", config.has_backpressure());
        println!("  - Timeout: {:?}", config.timeout());
        println!("  - Statistics interval: {:?}", config.statistics_interval);

        // Quick demonstration with a simple event
        let (sender, receiver) = async_channel::bounded::<String>(1);
        let event_receiver = AsyncEventReceiver::new(receiver, config);

        sender.send("test".to_string()).await.unwrap();
        drop(sender);

        if let Ok(event) = event_receiver.receive_event().await {
            println!("  - Successfully processed: '{}'", event);
            println!("  - Events count: {}", event_receiver.received_events_count());
        }
    }

    Ok(())
}