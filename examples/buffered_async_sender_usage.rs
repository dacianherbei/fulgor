//! Example demonstrating BufferedAsyncSender usage with different precision types
//!
//! This example shows how to instantiate and use the templated BufferedAsyncSender
//! with various number types for precision-dependent operations.

use fulgor::renderer::prelude::*;
use std::time::Duration;
use tokio;
use fulgor::renderer::RendererEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("fulgor BufferedAsyncSender Usage Examples");
    println!("========================================");

    // Example 1: Using f64 precision (default)
    println!("\n1. Creating BufferedAsyncSender with f64 precision:");

    let config_f64 = AsyncChannelConfig::<f64> {
        maximum_buffer_size: 1000,
        send_timeout: Some(Duration::from_millis(50)),
        enable_backpressure: false,
        statistics_interval: Duration::from_secs(1),
        precision_threshold: 0.001f64, // High precision threshold
    };

    let (sender_f64, mut receiver_f64) = BufferedAsyncSender::new(config_f64);

    println!("  - Channel created with {} max buffer size",
             sender_f64.configuration().maximum_buffer_size);
    println!("  - Precision threshold: {}",
             sender_f64.configuration().precision_threshold);

    // Example 2: Using f32 precision for performance-critical scenarios
    println!("\n2. Creating BufferedAsyncSender with f32 precision:");

    let config_f32 = AsyncChannelConfig::<f32> {
        maximum_buffer_size: 2000,
        send_timeout: Some(Duration::from_millis(25)),
        enable_backpressure: true, // Enable backpressure for this example
        statistics_interval: Duration::from_millis(500),
        precision_threshold: 0.01f32, // Lower precision, better performance
    };

    let (sender_f32, mut receiver_f32) = BufferedAsyncSender::new(config_f32);

    println!("  - Channel created with {} max buffer size",
             sender_f32.configuration().maximum_buffer_size);
    println!("  - Precision threshold: {}",
             sender_f32.configuration().precision_threshold);
    println!("  - Backpressure enabled: {}",
             sender_f32.configuration().enable_backpressure);

    // Example 3: Demonstrating mathematical operations on config
    println!("\n3. Mathematical operations on configuration:");

    let math_result_f64 = sender_f64.configuration() + 0.005f64;
    let math_result_f32 = sender_f32.configuration() * 2.0f32;

    println!("  - F64 config + 0.005 = {}", math_result_f64);
    println!("  - F32 config * 2.0 = {}", math_result_f32);

    // Example 4: Sending events
    println!("\n4. Sending renderer events:");

    // Send some events asynchronously
    let event1 = RendererEvent::Started(RendererKind::CpuReference);
    let event2 = RendererEvent::FrameRendered {
        renderer_kind: RendererKind::CpuReference,
        frame_number: 0,
        frame_time_microseconds: 16667, // ~60 FPS
        render_time_ns: 0,
    };

    // Send with f64 sender
    match sender_f64.send_event(event1.clone()).await {
        Ok(()) => println!("  - Event sent successfully via f64 sender"),
        Err(e) => println!("  - Failed to send via f64 sender: {:?}", e),
    }

    // Try send with f32 sender (non-blocking)
    match sender_f32.try_send_event(event2.clone()) {
        Ok(()) => println!("  - Event sent successfully via f32 sender (try_send)"),
        Err(e) => println!("  - Failed to try_send via f32 sender: {:?}", e),
    }

    // Example 5: Receiving events
    println!("\n5. Receiving events:");

    // Receive from f64 receiver
    tokio::select! {
        result = receiver_f64.receive_event() => {
            match result {
                Ok(event) => println!("  - Received from f64 receiver: {:?}", event),
                Err(e) => println!("  - Error receiving from f64 receiver: {:?}", e),
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            println!("  - Timeout waiting for f64 receiver");
        }
    }

    // Try receive from f32 receiver (non-blocking)
    match receiver_f32.try_receive_event() {
        Ok(event) => println!("  - Received from f32 receiver: {:?}", event),
        Err(async_channel::TryRecvError::Empty) => {
            println!("  - F32 receiver is empty");
        }
        Err(e) => println!("  - Error trying to receive from f32 receiver: {:?}", e),
    }

    // Example 6: Statistics
    println!("\n6. Channel statistics:");
    println!("  - F64 sender dropped events: {}", sender_f64.dropped_events_count());
    println!("  - F64 sender pending events: {}", sender_f64.pending_events_count());
    println!("  - F64 sender channel closed: {}", sender_f64.is_channel_closed());

    println!("  - F32 sender dropped events: {}", sender_f32.dropped_events_count());
    println!("  - F32 sender pending events: {}", sender_f32.pending_events_count());
    println!("  - F32 sender channel closed: {}", sender_f32.is_channel_closed());

    println!("  - F64 receiver received count: {}", receiver_f64.received_events_count());
    println!("  - F32 receiver received count: {}", receiver_f32.received_events_count());

    println!("\nExample completed successfully!");
    Ok(())
}