use std::time::Duration;

/// Configuration for asynchronous channel behavior and buffering strategies.
///
/// This struct defines how an async channel should handle buffering and
/// overflow scenarios when the channel becomes full.
/// Configuration for async channel behavior
#[derive(Debug, Clone)]
pub struct AsyncChannelConfig<NumberType = f64>
where
    NumberType: Copy + Clone + Send + Sync + 'static,
{
    /// Maximum buffer size before events start getting dropped
    pub maximum_buffer_size: usize,
    /// Timeout for send operations
    pub send_timeout: Option<Duration>,
    /// Whether to enable backpressure or drop events when full
    pub enable_backpressure: bool,
    /// Statistics collection interval
    pub statistics_interval: Duration,
    /// Precision-dependent threshold (templated for future use)
    pub precision_threshold: NumberType,
}

impl<NumberType> Default for AsyncChannelConfig<NumberType>
where
    NumberType: Copy + Clone + Send + Sync + Default + 'static,
{
    /// Creates a default configuration with reasonable defaults.
    ///
    /// Default values:
    /// - `maximum_buffer_size`: 1000 events
    /// - `send_timeout`: 100ms
    /// - `enable_backpressure`: false (drop oldest events)
    /// - `statistics_interval`: 1 second
    /// - `precision_threshold`: NumberType::default()
    fn default() -> Self {
        Self {
            maximum_buffer_size: 1000,
            send_timeout: Some(Duration::from_millis(100)),
            enable_backpressure: false,
            statistics_interval: Duration::from_secs(1),
            precision_threshold: NumberType::default(),
        }
    }
}

impl<NumberType> AsyncChannelConfig<NumberType>
where
    NumberType: Copy + Clone + Send + Sync + 'static,
{
    /// Creates a new configuration with all parameters explicitly specified.
    ///
    /// # Parameters
    ///
    /// * `maximum_buffer_size` - Maximum number of events to buffer
    /// * `send_timeout` - Optional timeout for send operations
    /// * `enable_backpressure` - Whether to enable backpressure vs dropping events
    /// * `statistics_interval` - How often to collect performance statistics
    /// * `precision_threshold` - Numerical precision threshold for operations
    ///
    /// # Returns
    ///
    /// A new AsyncChannelConfig instance with the specified parameters
    pub fn new(
        maximum_buffer_size: usize,
        send_timeout: Option<Duration>,
        enable_backpressure: bool,
        statistics_interval: Duration,
        precision_threshold: NumberType,
    ) -> Self {
        Self {
            maximum_buffer_size,
            send_timeout,
            enable_backpressure,
            statistics_interval,
            precision_threshold,
        }
    }

    /// Creates an unbounded channel configuration.
    ///
    /// This configuration uses a very large buffer size and disables timeouts
    /// and backpressure for maximum throughput.
    ///
    /// # Returns
    ///
    /// Configuration optimized for unbounded operation
    pub fn unbounded() -> Self
    where
        NumberType: Default,
    {
        Self {
            maximum_buffer_size: usize::MAX,
            send_timeout: None,
            enable_backpressure: false,
            statistics_interval: Duration::from_secs(5),
            precision_threshold: NumberType::default(),
        }
    }

    /// Creates a bounded channel configuration with the specified buffer size.
    ///
    /// Uses default values for other parameters with drop-oldest behavior.
    ///
    /// # Parameters
    ///
    /// * `buffer_size` - Maximum number of events to buffer
    ///
    /// # Returns
    ///
    /// Bounded configuration that drops oldest events when full
    pub fn bounded(buffer_size: usize) -> Self
    where
        NumberType: Default,
    {
        Self {
            maximum_buffer_size: buffer_size,
            send_timeout: Some(Duration::from_millis(100)),
            enable_backpressure: false,
            statistics_interval: Duration::from_secs(1),
            precision_threshold: NumberType::default(),
        }
    }

    /// Creates a bounded channel that applies backpressure when full.
    ///
    /// Instead of dropping events, this configuration will block senders
    /// when the buffer reaches capacity.
    ///
    /// # Parameters
    ///
    /// * `buffer_size` - Maximum number of events to buffer
    ///
    /// # Returns
    ///
    /// Bounded configuration with backpressure enabled
    pub fn bounded_with_backpressure(buffer_size: usize) -> Self
    where
        NumberType: Default,
    {
        Self {
            maximum_buffer_size: buffer_size,
            send_timeout: Some(Duration::from_millis(200)),
            enable_backpressure: true,
            statistics_interval: Duration::from_secs(1),
            precision_threshold: NumberType::default(),
        }
    }

    /// Creates a low-latency configuration optimized for real-time applications.
    ///
    /// Uses small buffer, short timeouts, and frequent statistics collection.
    ///
    /// # Parameters
    ///
    /// * `precision_threshold` - Precision threshold for numerical operations
    ///
    /// # Returns
    ///
    /// Configuration optimized for low latency
    pub fn low_latency(precision_threshold: NumberType) -> Self {
        Self {
            maximum_buffer_size: 100,
            send_timeout: Some(Duration::from_millis(10)),
            enable_backpressure: true,
            statistics_interval: Duration::from_millis(100),
            precision_threshold,
        }
    }

    /// Creates a high-throughput configuration optimized for batch processing.
    ///
    /// Uses large buffer, longer timeouts, and less frequent statistics collection.
    ///
    /// # Parameters
    ///
    /// * `precision_threshold` - Precision threshold for numerical operations
    ///
    /// # Returns
    ///
    /// Configuration optimized for high throughput
    pub fn high_throughput(precision_threshold: NumberType) -> Self {
        Self {
            maximum_buffer_size: 10000,
            send_timeout: Some(Duration::from_secs(1)),
            enable_backpressure: false,
            statistics_interval: Duration::from_secs(10),
            precision_threshold,
        }
    }

    /// Returns whether this configuration represents an unbounded channel.
    ///
    /// # Returns
    ///
    /// `true` if the buffer size is effectively unlimited
    pub fn is_unbounded(&self) -> bool {
        self.maximum_buffer_size == usize::MAX
    }

    /// Returns whether backpressure is enabled.
    ///
    /// # Returns
    ///
    /// `true` if backpressure is enabled, `false` if events are dropped
    pub fn has_backpressure(&self) -> bool {
        self.enable_backpressure
    }

    /// Returns the effective timeout duration for operations.
    ///
    /// # Returns
    ///
    /// The configured timeout, or None if operations should never timeout
    pub fn timeout(&self) -> Option<Duration> {
        self.send_timeout
    }

    /// Gets the precision threshold for numerical operations.
    ///
    /// # Returns
    ///
    /// The configured precision threshold
    pub fn precision_threshold(&self) -> NumberType {
        self.precision_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = AsyncChannelConfig::<f64>::default();
        assert_eq!(config.maximum_buffer_size, 1000);
        assert_eq!(config.send_timeout, Some(Duration::from_millis(100)));
        assert_eq!(config.enable_backpressure, false);
        assert_eq!(config.statistics_interval, Duration::from_secs(1));
        assert_eq!(config.precision_threshold, 0.0f64);
    }

    #[test]
    fn test_unbounded_configuration() {
        let config = AsyncChannelConfig::<f32>::unbounded();
        assert_eq!(config.maximum_buffer_size, usize::MAX);
        assert_eq!(config.send_timeout, None);
        assert_eq!(config.enable_backpressure, false);
        assert!(config.is_unbounded());
    }

    #[test]
    fn test_bounded_configuration() {
        let config = AsyncChannelConfig::<f64>::bounded(500);
        assert_eq!(config.maximum_buffer_size, 500);
        assert_eq!(config.enable_backpressure, false);
        assert!(!config.is_unbounded());
        assert!(!config.has_backpressure());
    }

    #[test]
    fn test_bounded_with_backpressure_configuration() {
        let config = AsyncChannelConfig::<f32>::bounded_with_backpressure(250);
        assert_eq!(config.maximum_buffer_size, 250);
        assert_eq!(config.enable_backpressure, true);
        assert!(config.has_backpressure());
    }

    #[test]
    fn test_low_latency_configuration() {
        let config = AsyncChannelConfig::low_latency(0.001f64);
        assert_eq!(config.maximum_buffer_size, 100);
        assert_eq!(config.send_timeout, Some(Duration::from_millis(10)));
        assert_eq!(config.enable_backpressure, true);
        assert_eq!(config.precision_threshold, 0.001f64);
    }

    #[test]
    fn test_high_throughput_configuration() {
        let config = AsyncChannelConfig::high_throughput(0.01f32);
        assert_eq!(config.maximum_buffer_size, 10000);
        assert_eq!(config.send_timeout, Some(Duration::from_secs(1)));
        assert_eq!(config.enable_backpressure, false);
        assert_eq!(config.precision_threshold, 0.01f32);
    }

    #[test]
    fn test_custom_configuration() {
        let config = AsyncChannelConfig::new(
            2000,
            Some(Duration::from_millis(50)),
            true,
            Duration::from_secs(2),
            0.005f64,
        );
        assert_eq!(config.maximum_buffer_size, 2000);
        assert_eq!(config.send_timeout, Some(Duration::from_millis(50)));
        assert_eq!(config.enable_backpressure, true);
        assert_eq!(config.statistics_interval, Duration::from_secs(2));
        assert_eq!(config.precision_threshold, 0.005f64);
    }

    #[test]
    fn test_configuration_methods() {
        let config = AsyncChannelConfig::<f64>::bounded_with_backpressure(100);

        assert!(!config.is_unbounded());
        assert!(config.has_backpressure());
        assert_eq!(config.timeout(), Some(Duration::from_millis(200)));
        assert_eq!(config.precision_threshold(), 0.0f64);
    }

    #[test]
    fn test_clone_and_debug() {
        let config = AsyncChannelConfig::low_latency(0.1f32);
        let cloned = config.clone();

        assert_eq!(config.maximum_buffer_size, cloned.maximum_buffer_size);
        assert_eq!(config.send_timeout, cloned.send_timeout);
        assert_eq!(config.enable_backpressure, cloned.enable_backpressure);
        assert_eq!(config.precision_threshold, cloned.precision_threshold);

        // Test Debug trait
        let debug_string = format!("{:?}", config);
        assert!(debug_string.contains("AsyncChannelConfig"));
    }

    #[test]
    fn test_different_number_types() {
        let f32_config = AsyncChannelConfig::<f32>::bounded(100);
        let f64_config = AsyncChannelConfig::<f64>::bounded(100);
        let i32_config = AsyncChannelConfig::<i32>::new(
            100,
            Some(Duration::from_millis(10)),
            false,
            Duration::from_secs(1),
            42i32,
        );

        assert_eq!(f32_config.precision_threshold, 0.0f32);
        assert_eq!(f64_config.precision_threshold, 0.0f64);
        assert_eq!(i32_config.precision_threshold, 42i32);
    }
}