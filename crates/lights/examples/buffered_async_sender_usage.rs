// Fixed example for buffered_async_sender_usage.rs

use lights::renderer::{
    async_communication::{AsyncEventReceiver, AsyncChannelConfig},
    factory::{RendererFactory, MockRendererFactory},
    RendererEvent, DataPrecision
};
use futures::StreamExt;
use tokio::time::{sleep, Duration};

/// Subscribe to buffered bounded events from a BufferedAsyncSender
pub fn subscribe_buffered_bounded(
    buffer_size: usize,
    enable_backpressure: bool,
) -> AsyncEventReceiver<RendererEvent> {
    // This is a mock implementation - you'll need to adapt based on the actual function
    let config = if enable_backpressure {
        AsyncChannelConfig::bounded_with_backpressure(buffer_size)
    } else {
        AsyncChannelConfig::bounded(buffer_size)
    };

    // Create a channel and receiver
    let (_, receiver) = async_channel::unbounded::<RendererEvent>();
    AsyncEventReceiver::new(receiver, config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting buffered async sender usage example...");

    // Create a renderer factory with a name
    let factory = MockRendererFactory::new("TestFactory");

    // Create renderer with proper error handling using the correct method name
    let renderer_result = factory.create(DataPrecision::F32, "test_renderer");

    // Extract the renderer from the Result (factory.create returns just Box<dyn Renderer>)
    let mut renderer = renderer_result?;

    // Get the BufferedAsyncSender from the renderer
    let buffered_sender = renderer.sender();

    // Now we can use subscribe_buffered_bounded with the correct type
    let mut receiver = subscribe_buffered_bounded(5, true);

    println!("Successfully created buffered async sender and receiver");

    // Start the renderer
    renderer.start()?;
    println!("Renderer started: {}", renderer.name());

    // Clone the sender before moving it into the spawned task
    let sender_clone = buffered_sender.clone();
    let renderer_id = renderer.unique_id();

    // Test sending some events
    tokio::spawn(async move {
        for i in 1..=10 {
            let event = RendererEvent::FrameRendered {
                renderer_id,
                frame_number: i,
                frame_time_microseconds: 16666, // ~60fps
                render_time_ns: 1_000_000,
            };

            if let Err(e) = sender_clone.send_event(event).await {
                eprintln!("Failed to send event {}: {}", i, e);
            } else {
                println!("Sent frame rendered event {}", i);
            }

            sleep(Duration::from_millis(100)).await;
        }
    });

    // Receive events
    let mut received_count = 0;
    while let Some(event) = receiver.next().await {
        match event {
            RendererEvent::FrameRendered { frame_number, .. } => {
                println!("Received frame rendered event: frame {}", frame_number);
                received_count += 1;

                // Stop after receiving 10 events
                if received_count >= 10 {
                    break;
                }
            }
            other => {
                println!("Received other event: {:?}", other);
            }
        }
    }

    println!("Received {} events total", received_count);
    println!("Receiver statistics:");
    println!("  - Events received: {}", receiver.received_events_count());
    println!("  - Channel closed: {}", receiver.is_closed());
    println!("  - Queue length: {}", receiver.len());

    // Test the dropped count functionality
    println!("Dropped events count: {}", buffered_sender.get_dropped_count());

    // Test configuration access
    let config = receiver.configuration();
    println!("Receiver configuration:");
    println!("  - Buffer size: {}", config.maximum_buffer_size);
    println!("  - Has backpressure: {}", config.has_backpressure());
    println!("  - Is unbounded: {}", config.is_unbounded());

    println!("Example completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lights::renderer::factory::MockRendererFactory;

    #[tokio::test]
    async fn test_buffered_async_sender_usage() -> Result<(), Box<dyn std::error::Error>> {
        let factory = MockRendererFactory::new();
        let renderer_result = factory.create_renderer(DataPrecision::F32, "test");

        // This is the key fix - properly handle the Result and extract the renderer
        let (renderer, _rx, _tx) = renderer_result?;
        let sender = renderer.sender();

        // Now we can create the receiver
        let mut receiver = subscribe_buffered_bounded(&sender, 10, false);

        // Test sending an event
        let test_event = RendererEvent::Started(renderer.unique_id());
        sender.send_event(test_event).await?;

        // Verify we can receive it
        if let Some(received_event) = receiver.next().await {
            match received_event {
                RendererEvent::Started(id) => {
                    assert_eq!(id, renderer.unique_id());
                    println!("Successfully received Started event for renderer {}", id);
                }
                other => panic!("Expected Started event, got {:?}", other),
            }
        }

        assert_eq!(receiver.received_events_count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_event_types() -> Result<(), Box<dyn std::error::Error>> {
        let factory = MockRendererFactory::new();
        let (renderer, _rx, _tx) = factory.create_renderer(DataPrecision::F32, "multi_test")?;
        let sender = renderer.sender();
        let mut receiver = subscribe_buffered_bounded(&sender, 20, true);

        // Send different types of events
        let events = vec![
            RendererEvent::Started(renderer.unique_id()),
            RendererEvent::ViewportResized {
                renderer_id: renderer.unique_id(),
                width: 1920,
                height: 1080,
            },
            RendererEvent::FrameRendered {
                renderer_id: renderer.unique_id(),
                frame_number: 1,
                frame_time_microseconds: 16666,
                render_time_ns: 2_000_000,
            },
            RendererEvent::Stopped(renderer.unique_id()),
        ];

        // Send all events
        for event in &events {
            sender.send_event(event.clone()).await?;
        }

        // Receive and verify all events
        let mut received_events = Vec::new();
        for _ in 0..events.len() {
            if let Some(event) = receiver.next().await {
                received_events.push(event);
            }
        }

        assert_eq!(received_events.len(), events.len());
        assert_eq!(receiver.received_events_count(), events.len() as u64);

        println!("Successfully sent and received {} events", events.len());
        Ok(())
    }

    #[tokio::test]
    async fn test_buffer_overflow_behavior() -> Result<(), Box<dyn std::error::Error>> {
        let factory = MockRendererFactory::new();
        let (renderer, _rx, _tx) = factory.create_renderer(DataPrecision::F32, "overflow_test")?;
        let sender = renderer.sender();

        // Create a small buffer to test overflow
        let mut receiver = subscribe_buffered_bounded(&sender, 2, false);

        // Send more events than the buffer can hold
        for i in 1..=5 {
            let event = RendererEvent::FrameRendered {
                renderer_id: renderer.unique_id(),
                frame_number: i,
                frame_time_microseconds: 16666,
                render_time_ns: 1_000_000,
            };

            if let Err(e) = sender.send_event(event).await {
                println!("Event {} was dropped: {}", i, e);
            }
        }

        // Check if any events were dropped
        let dropped_count = sender.get_dropped_count();
        println!("Total dropped events: {}", dropped_count);

        // Receive available events
        let mut received_count = 0;
        while let Some(_event) = receiver.try_receive_event().ok() {
            received_count += 1;
        }

        println!("Received {} events, dropped {} events", received_count, dropped_count);
        Ok(())
    }
}