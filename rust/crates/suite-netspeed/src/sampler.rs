use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::resolver::AdapterResolver;
use crate::types::{InterfaceSnapshot, InterfaceTable, RateCalculator, RateSample};

type SampleCallback = Arc<dyn Fn(RateSample) + Send + Sync>;

struct SamplerState {
    preferred_if_index: Option<u32>,
    preferred_alias: Option<String>,
    previous: Option<InterfaceSnapshot>,
    previous_table: Option<Vec<InterfaceSnapshot>>,
    previous_instant: Option<Instant>,
    stop: bool,
}

pub struct NetSpeedSampler {
    table: Arc<dyn InterfaceTable>,
    interval: Duration,
    state: Arc<(Mutex<SamplerState>, Condvar)>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl NetSpeedSampler {
    pub fn new(table: Arc<dyn InterfaceTable>, interval: Duration) -> Self {
        Self {
            table,
            interval,
            state: Arc::new((
                Mutex::new(SamplerState {
                    preferred_if_index: None,
                    preferred_alias: None,
                    previous: None,
                    previous_table: None,
                    previous_instant: None,
                    stop: false,
                }),
                Condvar::new(),
            )),
            worker: Mutex::new(None),
        }
    }

    pub fn set_preferred_adapter(&self, if_index: Option<u32>, alias: Option<String>) {
        let (lock, _) = &*self.state;
        let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        guard.preferred_if_index = if_index;
        guard.preferred_alias = alias;
        guard.previous = None;
        guard.previous_table = None;
        guard.previous_instant = None;
    }

    pub fn reset_after_resume(&self) {
        let (lock, _) = &*self.state;
        let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        guard.previous = None;
        guard.previous_table = None;
        guard.previous_instant = None;
    }

    pub fn list_adapters(&self) -> Result<Vec<InterfaceSnapshot>, String> {
        self.table.get_table()
    }

    pub fn start(&self, callback: SampleCallback) {
        let mut worker = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        if worker.is_some() {
            return;
        }
        {
            let (lock, _) = &*self.state;
            let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            guard.stop = false;
        }

        let table = Arc::clone(&self.table);
        let state = Arc::clone(&self.state);
        let interval = self.interval;
        let handle = std::thread::Builder::new()
            .name("suite-netspeed-sampler".to_string())
            .spawn(move || {
                Self::sample_once(&table, &state, &callback);
                let (lock, cv) = &*state;
                loop {
                    let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.stop {
                        break;
                    }
                    let (guard, _) = cv.wait_timeout(guard, interval).unwrap_or_else(|e| e.into_inner());
                    if guard.stop {
                        break;
                    }
                    drop(guard);
                    Self::sample_once(&table, &state, &callback);
                }
            })
            .expect("spawn netspeed sampler thread");
        *worker = Some(handle);
    }

    pub fn stop(&self) {
        let handle = {
            let mut worker = self.worker.lock().unwrap_or_else(|e| e.into_inner());
            worker.take()
        };
        let (lock, cv) = &*self.state;
        {
            let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            guard.stop = true;
            cv.notify_all();
        }
        if let Some(h) = handle {
            let _ = h.join();
        }
    }

    fn sample_once(
        table_src: &Arc<dyn InterfaceTable>,
        state: &Arc<(Mutex<SamplerState>, Condvar)>,
        callback: &SampleCallback,
    ) {
        let Ok(table) = table_src.get_table() else {
            callback(RateSample {
                if_index: 0,
                alias: String::new(),
                receive_bytes_per_second: 0.0,
                send_bytes_per_second: 0.0,
                adapter_missing: true,
            });
            return;
        };

        let (pref_idx, pref_alias, prev, prev_table, prev_instant) = {
            let (lock, _) = &**state;
            let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            (
                guard.preferred_if_index,
                guard.preferred_alias.clone(),
                guard.previous.clone(),
                guard.previous_table.clone(),
                guard.previous_instant,
            )
        };

        let resolved = AdapterResolver::resolve(
            &table,
            pref_idx,
            pref_alias.as_deref(),
            prev_table.as_deref(),
        )
        .cloned();

        let current = resolved
            .as_ref()
            .and_then(|r| table_src.get_entry(r.if_index).or_else(|| Some(r.clone())));

        let Some(current) = current else {
            let (lock, _) = &**state;
            let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            guard.previous = None;
            callback(RateSample {
                if_index: pref_idx.unwrap_or(0),
                alias: pref_alias.unwrap_or_default(),
                receive_bytes_per_second: 0.0,
                send_bytes_per_second: 0.0,
                adapter_missing: true,
            });
            return;
        };

        let now = Instant::now();
        let (receive, send) = match (prev, prev_instant) {
            (Some(p), Some(t)) => {
                let elapsed = now.duration_since(t).as_secs_f64();
                RateCalculator::try_compute(&p, &current, elapsed).unwrap_or((0.0, 0.0))
            }
            _ => (0.0, 0.0),
        };

        {
            let (lock, _) = &**state;
            let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            guard.previous = Some(current.clone());
            guard.previous_table = Some(table);
            guard.previous_instant = Some(now);
            guard.preferred_if_index = Some(current.if_index);
            guard.preferred_alias = Some(current.alias.clone());
        }

        callback(RateSample {
            if_index: current.if_index,
            alias: current.alias,
            receive_bytes_per_second: receive,
            send_bytes_per_second: send,
            adapter_missing: false,
        });
    }
}

impl Drop for NetSpeedSampler {
    fn drop(&mut self) {
        self.stop();
    }
}
