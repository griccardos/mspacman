use std::time::Duration;

pub struct TimedString {
    content: String,
    timestamp: std::time::Instant,
    duration: Duration,
}

impl TimedString {
    pub fn new(content: &str, duration: Duration) -> Self {
        Self {
            content: content.to_string(),
            duration,
            timestamp: std::time::Instant::now(),
        }
    }
    pub fn length(&self) -> usize {
        if self.timestamp.elapsed() > self.duration {
            0
        } else {
            self.content.len()
        }
    }
}

impl AsRef<str> for TimedString {
    fn as_ref(&self) -> &str {
        if self.timestamp.elapsed() > self.duration {
            ""
        } else {
            &self.content
        }
    }
}
