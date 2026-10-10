use std::{
    collections::{HashMap, VecDeque},
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

const DEFAULT_MAX_REQUESTS: usize = 5;
const DEFAULT_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub(crate) struct ContactRateLimiter {
    max_requests: usize,
    window: Duration,
    hits: Mutex<HashMap<IpAddr, VecDeque<Instant>>>,
}

impl Default for ContactRateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_REQUESTS, DEFAULT_WINDOW)
    }
}

impl ContactRateLimiter {
    pub(crate) fn new(max_requests: usize, window: Duration) -> Self {
        Self { max_requests, window, hits: Mutex::new(HashMap::new()) }
    }

    pub(crate) fn check_and_record(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().unwrap_or_else(|error| error.into_inner());
        let entries = hits.entry(ip).or_default();

        while entries.front().is_some_and(|instant| now.duration_since(*instant) > self.window) {
            entries.pop_front();
        }

        if entries.len() >= self.max_requests {
            return false;
        }

        entries.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn allows_requests_under_the_limit() {
        let limiter = ContactRateLimiter::new(2, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);

        assert!(limiter.check_and_record(ip));
        assert!(limiter.check_and_record(ip));
        assert!(!limiter.check_and_record(ip));
    }
}
