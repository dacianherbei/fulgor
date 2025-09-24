use std::time::Duration;

/// Configuration for asynchronous channel behavior and buffering strategies.
///
/// This struct defines how an async channel should handle buffering and
/// overflow scenarios when the channel becomes full.
/// Configuration for async channel behavior
#[derive(Debug, Clone)]
pub struct AsyncChannelConfig {
    /// Maximum buffer size before events start getting dropped
    pub maximum_buffer_size: usize,
    /// Timeout for send operations
    pub send_timeout: Option<Duration>,
    /// Whether to enable backpressure or drop events when full
    pub enable_backpressure: bool,
    /// Statistics collection interval
    pub statistics_interval: Duration,
}

impl Default for AsyncChannelConfig {
    /// Creates a default configuration with reasonable defaults.
    ///
    /// Default values:
    /// - `maximum_buffer_size`: 1000 events
    /// - `send_timeout`: 100ms
    /// - `enable_backpressure`: false (drop oldest events)
    /// - `statistics_interval`: 1 second
    fn default() -> Self {
        Self {
            maximum_buffer_size: 1000,
            send_timeout: Some(Duration::from_millis(100)),
            enable_backpressure: false,
            statistics_interval: Duration::from_secs(1)
        }
    }
}

impl AsyncChannelConfig {
    /// Creates a new configuration with all parameters explicitly specified.
    ///
    /// # Parameters
    ///
    /// * `maximum_buffer_size` - Maximum number of events to buffer
    /// * `send_timeout` - Optional timeout for send operations
    /// * `enable_backpressure` - Whether to enable backpressure vs dropping events
    /// * `statistics_interval` - How often to collect performance statistics
    ///
    /// # Returns
    ///
    /// A new AsyncChannelConfig instance with the specified parameters
    pub fn new(
        maximum_buffer_size: usize,
        send_timeout: Option<Duration>,
        enable_backpressure: bool,
        statistics_interval: Duration
    ) -> Self {
        Self {
            maximum_buffer_size,
            send_timeout,
            enable_backpressure,
            statistics_interval
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
    pub fn unbounded() -> Self {
        Self {
            maximum_buffer_size: usize::MAX,
            send_timeout: None,
            enable_backpressure: false,
            statistics_interval: Duration::from_secs(5)
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
    pub fn bounded(buffer_size: usize) -> Self {
        Self {
            maximum_buffer_size: buffer_size,
            send_timeout: Some(Duration::from_millis(100)),
            enable_backpressure: false,
            statistics_interval: Duration::from_secs(1)
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
    pub fn bounded_with_backpressure(buffer_size: usize) -> Self {
        Self {
            maximum_buffer_size: buffer_size,
            send_timeout: Some(Duration::from_millis(200)),
            enable_backpressure: true,
            statistics_interval: Duration::from_secs(1)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = AsyncChannelConfig::default();
        assert_eq!(config.maximum_buffer_size, 1000);
        assert_eq!(config.send_timeout, Some(Duration::from_millis(100)));
        assert_eq!(config.enable_backpressure, false);
        assert_eq!(config.statistics_interval, Duration::from_secs(1));
    }

    #[test]
    fn test_unbounded_configuration() {
        let config = AsyncChannelConfig::unbounded();
        assert_eq!(config.maximum_buffer_size, usize::MAX);
        assert_eq!(config.send_timeout, None);
        assert_eq!(config.enable_backpressure, false);
        assert!(config.is_unbounded());
    }

    #[test]
    fn test_bounded_configuration() {
        let config = AsyncChannelConfig::bounded(500);
        assert_eq!(config.maximum_buffer_size, 500);
        assert_eq!(config.enable_backpressure, false);
        assert!(!config.is_unbounded());
        assert!(!config.has_backpressure());
    }

    #[test]
    fn test_bounded_with_backpressure_configuration() {
        let config = AsyncChannelConfig::bounded_with_backpressure(250);
        assert_eq!(config.maximum_buffer_size, 250);
        assert_eq!(config.enable_backpressure, true);
        assert!(config.has_backpressure());
    }

    #[test]
    fn test_custom_configuration() {
        let config = AsyncChannelConfig::new(
            2000,
            Some(Duration::from_millis(50)),
            true,
            Duration::from_secs(2)
        );
        assert_eq!(config.maximum_buffer_size, 2000);
        assert_eq!(config.send_timeout, Some(Duration::from_millis(50)));
        assert_eq!(config.enable_backpressure, true);
        assert_eq!(config.statistics_interval, Duration::from_secs(2));
    }

    #[test]
    fn test_configuration_methods() {
        let config = AsyncChannelConfig::bounded_with_backpressure(100);

        assert!(!config.is_unbounded());
        assert!(config.has_backpressure());
        assert_eq!(config.timeout(), Some(Duration::from_millis(200)));
    }
}