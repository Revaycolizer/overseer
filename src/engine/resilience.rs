use std::future::Future;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct CircuitBreaker {
    failures: Arc<AtomicU32>,
    opened_at_ms: Arc<AtomicU64>,
    threshold: u32,
    open_ms: u64,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, open_ms: u64) -> Self {
        Self {
            failures: Arc::new(AtomicU32::new(0)),
            opened_at_ms: Arc::new(AtomicU64::new(0)),
            threshold,
            open_ms,
        }
    }
    pub fn is_open(&self) -> bool {
        let o = self.opened_at_ms.load(Ordering::Acquire);
        if o == 0 {
            return false;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if now.saturating_sub(o) >= self.open_ms {
            self.opened_at_ms.store(0, Ordering::Release);
            self.failures.store(0, Ordering::Release);
            return false;
        }
        true
    }
    fn record_success(&self) {
        self.failures.store(0, Ordering::Release);
    }
    fn record_failure(&self) {
        let n = self.failures.fetch_add(1, Ordering::AcqRel) + 1;
        if n >= self.threshold {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            self.opened_at_ms.store(now, Ordering::Release);
        }
    }
}

static OLLAMA_BREAKER: OnceLock<CircuitBreaker> = OnceLock::new();
pub fn ollama() -> &'static CircuitBreaker {
    OLLAMA_BREAKER.get_or_init(|| CircuitBreaker::new(5, 30_000))
}

static OLLAMA_DISPATCH_THROTTLE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
pub fn ollama_dispatch_throttle() -> &'static tokio::sync::Semaphore {
    OLLAMA_DISPATCH_THROTTLE.get_or_init(|| tokio::sync::Semaphore::new(4))
}

// Title-refinement is the slow path (1-3s on a small model) and only
// fires on first-message. A separate single-permit lane keeps a slow
// title call from holding one of the dispatch's four permits.
static OLLAMA_TITLE_THROTTLE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
pub fn ollama_title_throttle() -> &'static tokio::sync::Semaphore {
    OLLAMA_TITLE_THROTTLE.get_or_init(|| tokio::sync::Semaphore::new(1))
}

/// Backwards-compatible alias for ollama_dispatch_throttle.
pub fn ollama_throttle() -> &'static tokio::sync::Semaphore {
    ollama_dispatch_throttle()
}

type ServerGovernorConfig = tower_governor::governor::GovernorConfig<
    tower_governor::key_extractor::SmartIpKeyExtractor,
    governor::middleware::NoOpMiddleware<governor::clock::QuantaInstant>,
>;

static SERVER_GOVERNOR: OnceLock<Arc<ServerGovernorConfig>> = OnceLock::new();
pub fn server_governor() -> Arc<ServerGovernorConfig> {
    SERVER_GOVERNOR
        .get_or_init(|| {
            Arc::new(
                tower_governor::governor::GovernorConfigBuilder::default()
                    // This API takes a refill interval, not requests per second.
                    // Ten requests/second supports desktop setup and reload traffic.
                    .per_millisecond(100)
                    .burst_size(32)
                    .key_extractor(tower_governor::key_extractor::SmartIpKeyExtractor)
                    .finish()
                    .expect("valid governor config"),
            )
        })
        .clone()
}

fn backoff_ms(attempt: u32) -> u64 {
    let cap = 30_000u64;
    let exp = (200u64).saturating_mul(1u64 << attempt.min(20)).min(cap);
    let jitter = rand::random::<u64>() % (exp / 2 + 1);
    exp / 2 + jitter
}

pub enum ResilienceError<E> {
    CircuitOpen,
    Inner(E),
}

impl<E: std::fmt::Display> std::fmt::Display for ResilienceError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircuitOpen => write!(f, "circuit breaker open"),
            Self::Inner(e) => write!(f, "{}", e),
        }
    }
}

pub async fn with_resilience<T, E, F, Fut>(
    breaker: &CircuitBreaker,
    max_attempts: u32,
    mut f: F,
) -> Result<T, ResilienceError<E>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    if breaker.is_open() {
        return Err(ResilienceError::CircuitOpen);
    }
    let safe_attempts = max_attempts.max(1);
    for attempt in 0..safe_attempts {
        match f().await {
            Ok(v) => {
                breaker.record_success();
                return Ok(v);
            }
            Err(e) => {
                breaker.record_failure();
                if attempt + 1 < safe_attempts {
                    tokio::time::sleep(Duration::from_millis(backoff_ms(attempt))).await;
                } else {
                    return Err(ResilienceError::Inner(e));
                }
            }
        }
    }
    // Safety: the loop always returns on every iteration because safe_attempts >= 1.
    Err(ResilienceError::CircuitOpen)
}
#[cfg(test)]
mod server_rate_tests {
    use super::*;
    use governor::clock::{Clock, QuantaClock};

    #[test]
    fn desktop_traffic_has_fast_refill_and_a_bounded_burst() {
        let config = server_governor();
        let key = std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 42));
        for _ in 0..32 {
            config.limiter().check_key(&key).unwrap();
        }
        let limited = config.limiter().check_key(&key).unwrap_err();
        assert!(limited.wait_time_from(QuantaClock::default().now()) <= Duration::from_millis(100));
    }
}
