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
        Self { failures: Arc::new(AtomicU32::new(0)), opened_at_ms: Arc::new(AtomicU64::new(0)), threshold, open_ms }
    }
    pub fn is_open(&self) -> bool {
        let o = self.opened_at_ms.load(Ordering::Acquire);
        if o == 0 { return false; }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        if now.saturating_sub(o) >= self.open_ms {
            self.opened_at_ms.store(0, Ordering::Release);
            self.failures.store(0, Ordering::Release);
            return false;
        }
        true
    }
    fn record_success(&self) { self.failures.store(0, Ordering::Release); }
    fn record_failure(&self) {
        let n = self.failures.fetch_add(1, Ordering::AcqRel) + 1;
        if n >= self.threshold {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
            self.opened_at_ms.store(now, Ordering::Release);
        }
    }
}

static OLLAMA_BREAKER: OnceLock<CircuitBreaker> = OnceLock::new();
pub fn ollama() -> &'static CircuitBreaker { OLLAMA_BREAKER.get_or_init(|| CircuitBreaker::new(5, 30_000)) }

fn backoff_ms(attempt: u32) -> u64 {
    let cap = 30_000u64;
    let exp = (200u64).saturating_mul(1u64 << attempt.min(20)).min(cap);
    let jitter = rand::random::<u64>() % (exp / 2 + 1);
    exp / 2 + jitter
}

pub enum ResilienceError<E> { CircuitOpen, Inner(E) }

impl<E: std::fmt::Display> std::fmt::Display for ResilienceError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::CircuitOpen => write!(f, "circuit breaker open"), Self::Inner(e) => write!(f, "{}", e) }
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
    if breaker.is_open() { return Err(ResilienceError::CircuitOpen); }
    for attempt in 0..max_attempts {
        match f().await {
            Ok(v) => { breaker.record_success(); return Ok(v); }
            Err(e) => {
                breaker.record_failure();
                if attempt + 1 < max_attempts {
                    tokio::time::sleep(Duration::from_millis(backoff_ms(attempt))).await;
                } else { return Err(ResilienceError::Inner(e)); }
            }
        }
    }
    unreachable!()
}