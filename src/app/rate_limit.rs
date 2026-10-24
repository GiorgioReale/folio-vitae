use std::{
    collections::{HashMap, VecDeque},
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

const DEFAULT_MAX_REQUESTS: usize = 5;
const DEFAULT_WINDOW: Duration = Duration::from_secs(60);
/// Soft cap on tracked peer addresses to bound process memory growth.
const MAX_TRACKED_IPS: usize = 10_000;

#[derive(Debug)]
pub(crate) struct ContactRateLimiter {
    max_requests: usize,
    window: Duration,
    max_tracked_ips: usize,
    hits: Mutex<HashMap<IpAddr, VecDeque<Instant>>>,
}

impl Default for ContactRateLimiter {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_REQUESTS, DEFAULT_WINDOW)
    }
}

impl ContactRateLimiter {
    pub(crate) fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            max_tracked_ips: MAX_TRACKED_IPS,
            hits: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn check_and_record(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().unwrap_or_else(|error| error.into_inner());

        if !hits.contains_key(&ip) && hits.len() >= self.max_tracked_ips {
            Self::sweep_expired(&mut hits, now, self.window);
            if hits.len() >= self.max_tracked_ips {
                // Fail closed for previously unseen peers when the map is full.
                return false;
            }
        }

        let entries = hits.entry(ip).or_default();
        Self::prune_window(entries, now, self.window);

        if entries.len() >= self.max_requests {
            return false;
        }

        entries.push_back(now);
        true
    }

    fn prune_window(entries: &mut VecDeque<Instant>, now: Instant, window: Duration) {
        while entries.front().is_some_and(|instant| now.duration_since(*instant) > window) {
            entries.pop_front();
        }
    }

    fn sweep_expired(
        hits: &mut HashMap<IpAddr, VecDeque<Instant>>,
        now: Instant,
        window: Duration,
    ) {
        hits.retain(|_, entries| {
            Self::prune_window(entries, now, window);
            !entries.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn allows_requests_under_the_limit() {
        let limiter = ContactRateLimiter::new(2, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);

        assert!(limiter.check_and_record(ip));
        assert!(limiter.check_and_record(ip));
        assert!(!limiter.check_and_record(ip));
    }

    #[test]
    fn removes_inactive_keys_during_capacity_sweep() {
        let mut limiter = ContactRateLimiter::new(1, Duration::from_millis(1));
        limiter.max_tracked_ips = 2;

        let first = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let second = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        let third = IpAddr::V6(Ipv6Addr::LOCALHOST);

        assert!(limiter.check_and_record(first));
        assert!(limiter.check_and_record(second));
        std::thread::sleep(Duration::from_millis(5));

        // Expired keys are swept so a new address can be admitted.
        assert!(limiter.check_and_record(third));

        let hits = limiter.hits.lock().unwrap_or_else(|error| error.into_inner());
        assert!(hits.contains_key(&third));
        assert!(!hits.contains_key(&first));
        assert!(!hits.contains_key(&second));
    }
}
