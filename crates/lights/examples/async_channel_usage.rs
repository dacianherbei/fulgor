use fulgor::AsyncChannelConfig;

#[tokio::main]
async fn main() {
    // Default configuration
    let default_config = AsyncChannelConfig::default();
    println!("Default config: {:?}", default_config);

    // Unbounded channel
    let unbounded_config = AsyncChannelConfig::unbounded();
    println!("Unbounded config: {:?}", unbounded_config);

    // Bounded channel with specific size
    let bounded_config = AsyncChannelConfig::bounded(500);
    println!("Bounded config: {:?}", bounded_config);

    // Bounded channel that drops oldest messages
    let drop_oldest_config = AsyncChannelConfig::bounded_with_drop_oldest(100);
    println!("Drop oldest config: {:?}", drop_oldest_config);

    // Custom configuration
    let custom_config = AsyncChannelConfig::new(Some(2000), true);
    println!("Custom config: {:?}", custom_config);

    // Simulate async usage
    println!("\nSimulating async channel usage...");
    simulate_async_pipeline(default_config).await;
}

async fn simulate_async_pipeline(config: AsyncChannelConfig) {
    println!("Pipeline started with config: {:?}", config);

    // Simulate some async work
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    match config.buffer_size {
        Some(size) => println!("Processing with bounded buffer of size: {}", size),
        None => println!("Processing with unbounded buffer"),
    }

    if config.drop_oldest_on_full {
        println!("Buffer will drop oldest messages when full");
    } else {
        println!("Buffer will block when full");
    }

    println!("Pipeline completed successfully!");
}