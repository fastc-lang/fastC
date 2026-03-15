use axum::http::{HeaderMap, header};
use dashmap::DashMap;
use std::collections::VecDeque;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use crate::executor::ExecutorLimits;

/// Runtime configuration for the playground server.
#[derive(Debug, Clone)]
pub struct PlaygroundConfig {
    pub auth_token: Option<String>,
    pub allowed_origins: Vec<String>,
    pub max_request_body_bytes: usize,
    pub max_code_bytes: usize,
    pub max_runs_per_minute: usize,
    pub max_concurrent_runs: usize,
    pub executor_limits: ExecutorLimits,
}

impl Default for PlaygroundConfig {
    fn default() -> Self {
        Self {
            auth_token: None,
            allowed_origins: Vec::new(),
            max_request_body_bytes: 128 * 1024,
            max_code_bytes: 64 * 1024,
            max_runs_per_minute: 30,
            max_concurrent_runs: 4,
            executor_limits: ExecutorLimits::default(),
        }
    }
}

impl PlaygroundConfig {
    pub fn authorize(&self, headers: &HeaderMap) -> Result<(), String> {
        self.authorize_with(headers, None)
    }

    pub fn authorize_with(
        &self,
        headers: &HeaderMap,
        query_token: Option<&str>,
    ) -> Result<(), String> {
        let Some(expected) = self.auth_token.as_ref() else {
            return Ok(());
        };

        let bearer = headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));
        let custom = headers.get("x-fastc-token").and_then(|h| h.to_str().ok());
        let query = query_token;

        if bearer == Some(expected.as_str())
            || custom == Some(expected.as_str())
            || query == Some(expected.as_str())
        {
            Ok(())
        } else {
            Err("Unauthorized: missing or invalid playground token".to_string())
        }
    }
}

/// Sliding-window limiter keyed by remote IP address.
pub struct RunRateLimiter {
    max_runs: usize,
    window: Duration,
    entries: DashMap<IpAddr, VecDeque<Instant>>,
}

impl RunRateLimiter {
    pub fn new(max_runs: usize, window: Duration) -> Self {
        Self {
            max_runs,
            window,
            entries: DashMap::new(),
        }
    }

    pub fn allow(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut queue = self.entries.entry(ip).or_default();

        while let Some(oldest) = queue.front() {
            if now.duration_since(*oldest) > self.window {
                queue.pop_front();
            } else {
                break;
            }
        }

        if queue.len() >= self.max_runs {
            return false;
        }

        queue.push_back(now);
        true
    }

    pub fn cleanup(&self) {
        let now = Instant::now();
        self.entries.retain(|_, queue| {
            while let Some(oldest) = queue.front() {
                if now.duration_since(*oldest) > self.window {
                    queue.pop_front();
                } else {
                    break;
                }
            }
            !queue.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_blocks_after_limit() {
        let limiter = RunRateLimiter::new(2, Duration::from_secs(60));
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        assert!(limiter.allow(ip));
        assert!(limiter.allow(ip));
        assert!(!limiter.allow(ip));
    }
}
