/// Configuration for asynchronous channel behavior and buffering strategies.
///
/// This struct defines how an async channel should handle buffering and
/// overflow scenarios when the channel becomes full.
#[derive(Debug, Clone)]
pub struct AsyncChannelConfig {
    /// Optional buffer size for the channel. If None, the channel is unbounded.
    /// When Some(size), the channel will buffer up to `size` messages.
    pub buffer_size: Option<usize>,

    /// Behavior when the channel buffer is full and a new message arrives.
    /// - `true`: Drop the oldest message to make room for the new one
    /// - `false`: Block or return an error when trying to send to a full channel
    pub drop_oldest_on_full: bool,
}

impl Default for AsyncChannelConfig {
    /// Creates a default configuration with a buffer size of 1000 messages
    /// and blocking behavior when the buffer is full.
    fn default() -> Self {
        Self {
            buffer_size: Some(1000),
            drop_oldest_on_full: false,
        }
    }
}

impl AsyncChannelConfig {
    /// Creates a new configuration with specified buffer size and drop behavior.
    pub fn new(buffer_size: Option<usize>, drop_oldest_on_full: bool) -> Self {
        Self {
            buffer_size,
            drop_oldest_on_full,
        }
    }

    /// Creates an unbounded channel configuration.
    pub fn unbounded() -> Self {
        Self {
            buffer_size: None,
            drop_oldest_on_full: false,
        }
    }

    /// Creates a bounded channel configuration with the specified buffer size.
    pub fn bounded(size: usize) -> Self {
        Self {
            buffer_size: Some(size),
            drop_oldest_on_full: false,
        }
    }

    /// Creates a bounded channel that drops oldest messages when full.
    pub fn bounded_with_drop_oldest(size: usize) -> Self {
        Self {
            buffer_size: Some(size),
            drop_oldest_on_full: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = AsyncChannelConfig::default();
        assert_eq!(config.buffer_size, Some(1000));
        assert_eq!(config.drop_oldest_on_full, false);
    }

    #[test]
    fn test_unbounded_configuration() {
        let config = AsyncChannelConfig::unbounded();
        assert_eq!(config.buffer_size, None);
        assert_eq!(config.drop_oldest_on_full, false);
    }

    #[test]
    fn test_bounded_configuration() {
        let config = AsyncChannelConfig::bounded(500);
        assert_eq!(config.buffer_size, Some(500));
        assert_eq!(config.drop_oldest_on_full, false);
    }

    #[test]
    fn test_bounded_with_drop_oldest_configuration() {
        let config = AsyncChannelConfig::bounded_with_drop_oldest(250);
        assert_eq!(config.buffer_size, Some(250));
        assert_eq!(config.drop_oldest_on_full, true);
    }

    #[test]
    fn test_clone_and_debug() {
        let config = AsyncChannelConfig::default();
        let cloned = config.clone();
        assert_eq!(config.buffer_size, cloned.buffer_size);
        assert_eq!(config.drop_oldest_on_full, cloned.drop_oldest_on_full);

        // Test Debug trait
        let debug_string = format!("{:?}", config);
        assert!(debug_string.contains("AsyncChannelConfig"));
    }
}