//! Diagnostic transport shared by Spell Wheel and the DX12 hook. No render-thread file IO.

use std::cell::Cell;
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const EVENT_LIMIT: u64 = 4096;
const QUEUE_SIZE: usize = 256;
static TRACE: OnceLock<Trace> = OnceLock::new();
static PRESENT: AtomicU64 = AtomicU64::new(0);
thread_local! {
    static CONTEXT: Cell<(u64, bool)> = const { Cell::new((0, false)) };
}

struct Trace {
    start: Instant,
    tx: SyncSender<String>,
    sink: Mutex<(Receiver<String>, File)>,
    events: AtomicU64,
    dropped: AtomicU64,
    until_ms: AtomicU64,
    detail_left: AtomicU64,
    sample_ms: AtomicU64,
    flush_ms: AtomicU64,
}

/// Open a separate per-process/module trace. Call once on initialization, after checking opt-in.
pub fn enable(path: &Path) -> io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "settings-diagnostics-v1 pid={} limit={EVENT_LIMIT} queue={QUEUE_SIZE}; timestamps=monotonic_us; flush=game_tick_1s", std::process::id())?;
    let (tx, rx) = sync_channel(QUEUE_SIZE);
    TRACE.set(Trace {
        start: Instant::now(), tx, sink: Mutex::new((rx, file)),
        events: AtomicU64::new(0), dropped: AtomicU64::new(0),
        until_ms: AtomicU64::new(0), detail_left: AtomicU64::new(0),
        sample_ms: AtomicU64::new(0), flush_ms: AtomicU64::new(0),
    }).map_err(|_| io::Error::other("trace already enabled"))
}

fn reserve(counter: &AtomicU64, limit: u64) -> bool {
    counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
        (n < limit).then_some(n + 1)
    }).is_ok()
}

/// Queue one event without waiting for disk or queue capacity. Typed text must never be passed here.
pub fn event(args: fmt::Arguments<'_>) {
    let Some(trace) = TRACE.get() else { return };
    if !reserve(&trace.events, EVENT_LIMIT) { return; }
    let line = format!("us={} thread={:?} present={} {}", trace.start.elapsed().as_micros(),
        std::thread::current().id(), CONTEXT.with(|c| c.get().0), args);
    if trace.tx.try_send(line).is_err() {
        trace.dropped.fetch_add(1, Ordering::Relaxed);
    }
}

/// Capture detailed first 24 Presents and then 2 Hz samples for ten seconds after a release.
pub fn arm() {
    if let Some(trace) = TRACE.get() {
        trace.until_ms.store(trace.start.elapsed().as_millis() as u64 + 10_000, Ordering::Relaxed);
        trace.detail_left.store(24, Ordering::Relaxed);
    }
}

/// Assign a callback ID and select a bounded sample. Does not affect rendering decisions.
pub fn begin_present() -> PresentScope {
    let id = PRESENT.fetch_add(1, Ordering::Relaxed) + 1;
    let sample = TRACE.get().is_some_and(|trace| {
        let now = trace.start.elapsed().as_millis() as u64;
        if now >= trace.until_ms.load(Ordering::Relaxed) { return false; }
        if trace.detail_left.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
            return true;
        }
        trace.sample_ms.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |last| {
            (now >= last + 500).then_some(now)
        }).is_ok()
    });
    let previous = CONTEXT.with(|c| c.replace((id, sample)));
    PresentScope(previous)
}

/// Restores correlation if a chained hook reenters Present on the same thread.
pub struct PresentScope((u64, bool));

impl Drop for PresentScope {
    fn drop(&mut self) { CONTEXT.with(|c| c.set(self.0)); }
}

/// Whether the current Present was selected for draw/backend observations.
pub fn sampled() -> bool { CONTEXT.with(|c| c.get().1) }

/// Flush queued events at most once a second from the game tick, never from a render callback.
/// A stalled game tick can delay the final batch. Errors go to the normal log via the caller.
pub fn flush() -> io::Result<()> {
    let Some(trace) = TRACE.get() else { return Ok(()) };
    let now = trace.start.elapsed().as_millis() as u64;
    if trace.flush_ms.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |last| {
        (now >= last + 1000).then_some(now)
    }).is_err() { return Ok(()); }
    let Ok(mut sink) = trace.sink.try_lock() else { return Ok(()) };
    let (rx, file) = &mut *sink;
    // Bounded even if producers continue while we drain.
    let mut lines = rx.try_iter().take(QUEUE_SIZE).peekable();
    if lines.peek().is_none() { return Ok(()); }
    for line in lines { writeln!(file, "{line}")?; }
    writeln!(file, "flush_us={} attempted={} dropped={} capped={}", trace.start.elapsed().as_micros(),
        trace.events.load(Ordering::Relaxed), trace.dropped.load(Ordering::Relaxed),
        trace.events.load(Ordering::Relaxed) >= EVENT_LIMIT)?;
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_budget_is_exact_and_stays_bounded() {
        let counter = std::sync::Arc::new(AtomicU64::new(0));
        let handles: Vec<_> = (0..8).map(|_| {
            let counter = counter.clone();
            std::thread::spawn(move || (0..1000).filter(|_| reserve(&counter, 37)).count())
        }).collect();
        let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
        assert_eq!(total, 37);
        assert_eq!(counter.load(Ordering::Relaxed), 37);
    }

    #[test]
    fn nested_present_restores_callback_context() {
        CONTEXT.with(|c| c.set((999, false)));
        {
            let _outer = begin_present();
            let outer = CONTEXT.with(|c| c.get());
            {
                let _inner = begin_present();
                assert_ne!(CONTEXT.with(|c| c.get().0), outer.0);
            }
            assert_eq!(CONTEXT.with(|c| c.get()), outer);
        }
        assert_eq!(CONTEXT.with(|c| c.get()), (999, false));
    }

    #[test]
    fn opt_in_trace_drops_instead_of_blocking_when_queue_is_full() {
        let path = std::env::temp_dir().join(format!("spellwheel-trace-test-{}.log", std::process::id()));
        enable(&path).unwrap();
        for _ in 0..QUEUE_SIZE + 10 { event(format_args!("test-event")); }
        let trace = TRACE.get().unwrap();
        assert_eq!(trace.dropped.load(Ordering::Relaxed), 10);
        let mut sink = trace.sink.lock().unwrap();
        let (rx, file) = &mut *sink;
        for line in rx.try_iter() { writeln!(file, "{line}").unwrap(); }
        file.flush().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), QUEUE_SIZE + 1);
        drop(sink);
        // File remains open in TRACE on Windows; temp cleanup is left to the OS there.
        #[cfg(not(windows))]
        std::fs::remove_file(path).unwrap();
    }
}
