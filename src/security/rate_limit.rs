// Rate Limiting Module
use super::AgentId;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Rate limit configuration for a single agent
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub count: u32,
    pub window_start: Instant,
    pub max_requests: u32,
    pub window_duration: Duration,
}

impl RateLimit {
    pub fn new(max_requests: u32, window_duration: Duration) -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
            max_requests,
            window_duration,
        }
    }

    /// Check if request is allowed and increment counter
    pub fn check_and_increment(&mut self) -> bool {
        let now = Instant::now();

        // Reset window if expired
        if now.duration_since(self.window_start) >= self.window_duration {
            self.count = 0;
            self.window_start = now;
        }

        // Check limit
        if self.count >= self.max_requests {
            return false;
        }

        self.count += 1;
        true
    }

    /// Get remaining requests in current window
    pub fn remaining(&self) -> u32 {
        let now = Instant::now();
        
        if now.duration_since(self.window_start) >= self.window_duration {
            return self.max_requests;
        }
        
        self.max_requests.saturating_sub(self.count)
    }

    /// Get time until window reset
    pub fn reset_in(&self) -> Duration {
        let now = Instant::now();
        let elapsed = now.duration_since(self.window_start);
        
        if elapsed >= self.window_duration {
            Duration::from_secs(0)
        } else {
            self.window_duration - elapsed
        }
    }
}

/// Rate limiter for multiple agents
pub struct RateLimiter {
    limits: HashMap<AgentId, RateLimit>,
    default_max_requests: u32,
    default_window_duration: Duration,
    enabled: bool,
}

impl RateLimiter {
    /// Create a new rate limiter with default limits
    pub fn new(default_max_requests: u32, default_window_secs: u64) -> Self {
        Self {
            limits: HashMap::new(),
            default_max_requests,
            default_window_duration: Duration::from_secs(default_window_secs),
            enabled: true,
        }
    }

    /// Create with custom defaults
    pub fn with_defaults(
        default_max_requests: u32,
        default_window_duration: Duration,
    ) -> Self {
        Self {
            limits: HashMap::new(),
            default_max_requests,
            default_window_duration,
            enabled: true,
        }
    }

    /// Enable or disable rate limiting
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Set custom limit for an agent
    pub fn set_limit(&mut self, agent_id: AgentId, max_requests: u32, window_secs: u64) {
        self.limits.insert(
            agent_id,
            RateLimit::new(max_requests, Duration::from_secs(window_secs)),
        );
    }

    /// Check if request is allowed for an agent
    pub fn check(&mut self, agent_id: &AgentId) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let limit = self
            .limits
            .entry(agent_id.clone())
            .or_insert_with(|| {
                RateLimit::new(self.default_max_requests, self.default_window_duration)
            });

        if limit.check_and_increment() {
            Ok(())
        } else {
            Err(anyhow!(
                "Rate limit exceeded for agent {}. Try again in {:?}",
                agent_id,
                limit.reset_in()
            ))
        }
    }

    /// Get remaining requests for an agent
    pub fn remaining(&self, agent_id: &AgentId) -> u32 {
        if !self.enabled {
            return self.default_max_requests;
        }

        self.limits
            .get(agent_id)
            .map(|l| l.remaining())
            .unwrap_or(self.default_max_requests)
    }

    /// Get reset time for an agent
    pub fn reset_in(&self, agent_id: &AgentId) -> Option<Duration> {
        if !self.enabled {
            return None;
        }

        self.limits.get(agent_id).map(|l| l.reset_in())
    }

    /// Reset limit for an agent
    pub fn reset(&mut self, agent_id: &AgentId) {
        if let Some(limit) = self.limits.get_mut(agent_id) {
            limit.count = 0;
            limit.window_start = Instant::now();
        }
    }

    /// Clear all limits
    pub fn clear(&mut self) {
        self.limits.clear();
    }

    /// Get statistics for an agent
    pub fn stats(&self, agent_id: &AgentId) -> Option<RateLimitStats> {
        self.limits.get(agent_id).map(|limit| RateLimitStats {
            count: limit.count,
            max_requests: limit.max_requests,
            remaining: limit.remaining(),
            reset_in: limit.reset_in(),
        })
    }
}

/// Rate limit statistics
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub count: u32,
    pub max_requests: u32,
    pub remaining: u32,
    pub reset_in: Duration,
}

/// Token bucket rate limiter (alternative algorithm)
pub struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume tokens
    pub fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Get current token count
    pub fn available(&mut self) -> f64 {
        self.refill();
        self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_new() {
        let limit = RateLimit::new(10, Duration::from_secs(60));
        assert_eq!(limit.remaining(), 10);
    }

    #[test]
    fn test_rate_limit_check() {
        let mut limit = RateLimit::new(2, Duration::from_secs(60));
        
        assert!(limit.check_and_increment());
        assert!(limit.check_and_increment());
        assert!(!limit.check_and_increment()); // Exceeded
        
        assert_eq!(limit.remaining(), 0);
    }

    #[test]
    fn test_rate_limit_window_reset() {
        let mut limit = RateLimit::new(1, Duration::from_millis(10));
        
        assert!(limit.check_and_increment());
        assert!(!limit.check_and_increment());
        
        // Wait for window to reset
        std::thread::sleep(Duration::from_millis(15));
        
        assert!(limit.check_and_increment());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(2, 60);
        
        assert!(limiter.check(&"agent-1".to_string()).is_ok());
        assert!(limiter.check(&"agent-1".to_string()).is_ok());
        assert!(limiter.check(&"agent-1".to_string()).is_err());
        
        // Different agent should have separate limit
        assert!(limiter.check(&"agent-2".to_string()).is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let mut limiter = RateLimiter::new(1, 60);
        limiter.set_enabled(false);
        
        assert!(limiter.check(&"agent-1".to_string()).is_ok());
        assert!(limiter.check(&"agent-1".to_string()).is_ok());
        assert!(limiter.check(&"agent-1".to_string()).is_ok());
    }

    #[tokio::test]
    async fn test_custom_limit() {
        let mut limiter = RateLimiter::new(1, 60);
        limiter.set_limit("vip-agent".to_string(), 5, 60);
        
        // Default limit
        assert!(limiter.check(&"agent-1".to_string()).is_ok());
        assert!(limiter.check(&"agent-1".to_string()).is_err());
        
        // Custom limit
        for _ in 0..5 {
            assert!(limiter.check(&"vip-agent".to_string()).is_ok());
        }
        assert!(limiter.check(&"vip-agent".to_string()).is_err());
    }

    #[tokio::test]
    async fn test_reset() {
        let mut limiter = RateLimiter::new(1, 60);
        
        assert!(limiter.check(&"agent-1".to_string()).is_ok());
        assert!(limiter.check(&"agent-1".to_string()).is_err());
        
        limiter.reset(&"agent-1".to_string());
        
        assert!(limiter.check(&"agent-1".to_string()).is_ok());
    }

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 1.0); // 10 tokens, 1 per second
        
        assert!(bucket.try_consume(5.0));
        // Use approximate comparison for floating point
        assert!((bucket.available() - 5.0).abs() < 0.01);
        
        assert!(bucket.try_consume(5.0));
        assert!((bucket.available() - 0.0).abs() < 0.01);
        
        assert!(!bucket.try_consume(1.0)); // Not enough tokens
    }
}
