//! Usage example for BufferedAsyncSender
//!
//! This file would be placed in examples/buffered_async_sender_usage.rs

use std::any::Any;
use std::sync::{Arc};
use std::sync::atomic::AtomicU64;
use tokio::sync::mpsc::Receiver;
use fulgor::renderer::prelude::*;
use tokio::time::{sleep, Duration};

/// Subscribe using BufferedAsyncSender with bounded channel.
pub fn subscribe_buffered_bounded(
    buffered_async_sender:&BufferedAsyncSender<RendererEvent>,
    capacity: usize,
    drop_oldest_on_full: bool,
) -> Receiver<RendererEvent> {
    let (buffered_sender, receiver) = BufferedAsyncSender::<RendererEvent>::new_bounded(
        capacity,
        drop_oldest_on_full,
        Arc::new(AtomicU64::new(0)),
    );
    buffered_async_sender = buffered_sender;
    receiver
}

/// Subscribe using BufferedAsyncSender with unbounded channel.
pub fn subscribe_buffered_unbounded(buffered_async_sender:&BufferedAsyncSender<RendererEvent>) -> tokio::sync::mpsc::UnboundedReceiver<RendererEvent> {
    let (buffered_sender, receiver) =
        BufferedAsyncSender::<RendererEvent>::new_unbounded(Some(1));
    buffered_async_sender = buffered_sender;
    receiver
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("BufferedAsyncSender Usage Example");

    // Example 1: Bounded channel with drop_oldest_on_full = false
    println!("\n=== Example 1: Bounded Channel (drop newest on full) ===");
    bounded_channel_drop_newest_example().await?;

    // Example 2: Bounded channel with drop_oldest_on_full = true
    println!("\n=== Example 2: Bounded Channel (drop oldest on full) ===");
    bounded_channel_drop_oldest_example().await?;

    // Example 3: Unbounded channel
    println!("\n=== Example 3: Unbounded Channel ===");
    unbounded_channel_example().await?;

    // Example 4: Integration with RendererManager
    println!("\n=== Example 4: Integration with RendererManager ===");
    renderer_manager_integration_example().await?;

    Ok(())
}

async fn bounded_channel_drop_newest_example() -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut receiver) = BufferedAsyncSender::<RendererEvent>::new_bounded(3, false,Arc::new(AtomicU64::new(0)));

    println!("Created bounded channel with capacity 3, drop_newest_on_full = false");

    // Send events that will fill the channel
    for i in 0..5 {
        let rendererId = 1;
        let event = RendererEvent::Started(rendererId);
        let _ = sender.send_event(event).await;
        println!("Sent event {}", i + 1);
    }

    println!("Dropped events count: {}", sender.get_dropped_count());

    // Receive available events
    let mut received_count = 0;
    while let Ok(event) = receiver.try_recv() {
        received_count += 1;
        println!("Received event: {:?}", event);
    }

    println!("Total received: {}, Total dropped: {}", received_count, sender.get_dropped_count());

    Ok(())
}

async fn bounded_channel_drop_oldest_example() -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut receiver) = BufferedAsyncSender::<RendererEvent>::new_bounded(3, true,Arc::new(AtomicU64::new(0)));

    println!("Created bounded channel with capacity 3, drop_oldest_on_full = true");

    // Send events rapidly to test drop_oldest logic
    for i in 0..8 {
        let event = match i % 3 {
            0 => RendererEvent::Started(0),
            1 => RendererEvent::Stopped(1),
            _ => RendererEvent::Switched(Option::<u64>::Some(i.clone())),
        };

        let _ = sender.send_event(event).await;
        println!("Sent event {} (dropped count: {})", i + 1, sender.get_dropped_count());

        // Small delay to allow some processing
        sleep(Duration::from_millis(10)).await;
    }

    println!("Final dropped events count: {}", sender.get_dropped_count());

    // Receive remaining events
    let mut received_count = 0;
    while let Ok(event) = receiver.try_recv() {
        received_count += 1;
        println!("Received event: {:?}", event);
    }

    println!("Total received: {}", received_count);

    Ok(())
}

async fn unbounded_channel_example() -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut receiver) = BufferedAsyncSender::<RendererEvent>::new_unbounded(Option::<usize>::Some(1));

    println!("Created unbounded channel");

    // Send many events rapidly
    for i in 0..100 {
        let event = RendererEvent::Started(i);
        let _ = sender.send_event(event).await;

        if i % 20 == 0 {
            println!("Sent {} events (dropped count: {})", i + 1, sender.get_dropped_count());
        }
    }

    println!("Final dropped events count: {}", sender.get_dropped_count());

    // Receive events
    let mut received_count = 0;
    while let Ok(_event) = receiver.try_recv() {
        received_count += 1;
    }

    println!("Total received: {}", received_count);

    Ok(())
}

async fn renderer_manager_integration_example() -> Result<(), Box<dyn std::error::Error>> {
    let manager = RendererManager::new();
    let renderer = manager.create_by_name("ReferenceRendererFactory", DataPrecision::F64,"");
    // Subscribe using buffered async sender
    let mut receiver = subscribe_buffered_bounded(renderer,5, true);

    println!("Subscribed to RendererManager with buffered async sender");

    // Start and stop renderers asynchronously
    manager.start_async(renderer.iter().by_ref().unique_id()).await?;

    sleep(Duration::from_millis(50)).await;

    manager.stop_async(renderer.iter().by_ref().unique_id()).await;

    // Check dropped events
    if let Some(buffered_sender) = manager.get_buffered_sender() {
        println!("Dropped events from manager: {}", buffered_sender.get_dropped_count());
    }

    // Receive events
    let mut received_events = Vec::new();
    while let Ok(event) = receiver.try_recv() {
        received_events.push(event);
    }

    println!("Received {} events from RendererManager:", received_events.len());
    for (i, event) in received_events.iter().enumerate() {
        println!("  {}: {:?}", i + 1, event);
    }

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_send_event_bounded_channel() {
        let (sender, mut receiver) = BufferedAsyncSender::new_bounded(2, false,Arc::new(AtomicU64::new((0))));

        // Test normal sending
        let event1 = RendererEvent::Started(1);
        let event2 = RendererEvent::Stopped(1);

        sender.send_event(event1.clone()).await;
        sender.send_event(event2.clone()).await;

        // Should receive both events
        let received1 = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();
        let received2 = timeout(Duration::from_millis(100), receiver.recv()).await.unwrap().unwrap();

        assert!(matches!(received1, RendererEvent::Started(_)));
        assert!(matches!(received2, RendererEvent::Stopped(_)));
        assert_eq!(sender.get_dropped_count(), 0);
    }

    #[tokio::test]
    async fn test_send_event_channel_overflow() {
        let (sender, _receiver) = BufferedAsyncSender::new_bounded(2, false,Arc::new(AtomicU64::new((0))));

        // Fill the channel beyond capacity
        for i in 0..4 {
            let event = RendererEvent::Started(1);
            sender.send_event(event).await;
        }

        // Should have dropped events
        assert!(sender.get_dropped_count() > 0);
    }

    #[tokio::test]
    async fn test_send_event_unbounded_channel() {
        let (sender, mut receiver) = BufferedAsyncSender::new_unbounded(Option::<usize>::Some(1));

        // Send multiple events
        for i in 0..10 {
            let event = RendererEvent::Started(1);
            sender.send_event(event).await;
        }

        // Should not drop any events
        assert_eq!(sender.get_dropped_count(), 0);

        // Should be able to receive all events
        let mut received_count = 0;
        while receiver.try_recv().is_ok() {
            received_count += 1;
        }
        assert_eq!(received_count, 10);
    }

    #[tokio::test]
    async fn test_drop_oldest_on_full_logic() {
        let (sender, mut receiver) = BufferedAsyncSender::new_bounded(3, true, Arc::new(AtomicU64::new((0))));

        // Send more events than capacity
        for i in 0..6 {
            let event = RendererEvent::Started(1);
            sender.send_event(event).await;
        }

        // Give some time for async processing
        sleep(Duration::from_millis(50)).await;

        // Should have dropped some events due to overflow
        assert!(sender.get_dropped_count() > 0);
    }
}

/// Benchmark for performance testing
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn benchmark_bounded_channel_throughput() {
        let (sender, mut receiver) = BufferedAsyncSender::new_bounded(1000, true, Arc::new(AtomicU64::new((0))));
        let event_count = 10000;

        let start = Instant::now();

        // Send events
        for i in 0..event_count {
            let event = RendererEvent::Started(1);
            sender.send_event(event).await;
        }

        let send_duration = start.elapsed();

        // Receive events
        let mut received_count = 0;
        while receiver.try_recv().is_ok() {
            received_count += 1;
        }

        let total_duration = start.elapsed();

        println!("Benchmark Results:");
        println!("  Events sent: {}", event_count);
        println!("  Events received: {}", received_count);
        println!("  Events dropped: {}", sender.get_dropped_count());
        println!("  Send duration: {:?}", send_duration);
        println!("  Total duration: {:?}", total_duration);
        println!("  Throughput: {:.2} events/ms", event_count as f64 / send_duration.as_millis() as f64);
    }

    #[tokio::test]
    async fn benchmark_unbounded_channel_throughput() {
        let (sender, mut receiver) = BufferedAsyncSender::new_unbounded(Option::<usize>::Some(1));
        let event_count = 10000;

        let start = Instant::now();

        // Send events
        for i in 0..event_count {
            let event = RendererEvent::Started(1);
            sender.send_event(event).await;
        }

        let send_duration = start.elapsed();

        // Receive events
        let mut received_count = 0;
        while receiver.try_recv().is_ok() {
            received_count += 1;
        }

        let total_duration = start.elapsed();

        println!("Unbounded Benchmark Results:");
        println!("  Events sent: {}", event_count);
        println!("  Events received: {}", received_count);
        println!("  Events dropped: {}", sender.get_dropped_count());
        println!("  Send duration: {:?}", send_duration);
        println!("  Total duration: {:?}", total_duration);
        println!("  Throughput: {:.2} events/ms", event_count as f64 / send_duration.as_millis() as f64);
    }
}