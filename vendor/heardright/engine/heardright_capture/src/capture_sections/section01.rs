// Capture session: cpal::Stream + lock-free ring buffer + resampler.
//
// Threading model:
// - cpal owns an OS audio thread that calls our data callback. The callback
//   does only: convert sample → f32 mono → push into the SPSC ringbuf, then
//   atomic counter updates. No allocation, no logging, no Python.
// - Python (the consumer) calls `read*`. The consumer pops native-rate f32
//   samples out of the ring, runs them through the StreamResampler, and
//   converts to the requested output dtype.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, StreamConfig};
use ringbuf::{
    traits::{Consumer, Observer, Producer, Split},
    HeapRb,
};

use crate::device::resolve_device;
use crate::metrics::{CaptureErrorKind, Metrics, MetricsSnapshot};
use crate::resample::StreamResampler;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutDType {
    Int16,
    Float32,
}

impl OutDType {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "int16" | "i16" => Ok(OutDType::Int16),
            "float32" | "f32" => Ok(OutDType::Float32),
            other => Err(anyhow!(
                "unsupported dtype '{other}', want int16 or float32"
            )),
        }
    }
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            OutDType::Int16 => 2,
            OutDType::Float32 => 4,
        }
    }
}

pub struct SessionConfig {
    pub device_id: Option<usize>,
    pub target_rate: u32,
    pub target_channels: u16, // currently must be 1
    pub target_dtype: OutDType,
    pub block_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureInputWaitStatus {
    NoCallbacks,
    NoSamples,
    Ready,
}

fn classify_input_wait(
    callbacks_before: u64,
    callbacks_after: u64,
    output_samples: usize,
) -> CaptureInputWaitStatus {
    if output_samples > 0 {
        CaptureInputWaitStatus::Ready
    } else if callbacks_after == callbacks_before {
        CaptureInputWaitStatus::NoCallbacks
    } else {
        CaptureInputWaitStatus::NoSamples
    }
}

#[derive(Debug)]
pub enum CaptureFirstBuffer {
    Data(Vec<f32>),
    NoCallbacks,
    NoSamples,
    StreamError(CaptureErrorKind),
}

#[derive(Default)]
struct AudioArrival {
    epoch: AtomicU64,
    mutex: Mutex<()>,
    ready: Condvar,
}

impl AudioArrival {
    fn signal(&self) {
        self.epoch.fetch_add(1, Ordering::Release);
        self.ready.notify_one();
    }

    fn wait(&self, seen: &mut u64, timeout: Duration) -> bool {
        let current = self.epoch.load(Ordering::Acquire);
        if current != *seen {
            *seen = current;
            return true;
        }
        let guard = self.mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let (guard, _) = self
            .ready
            .wait_timeout_while(guard, timeout, |_| {
                self.epoch.load(Ordering::Acquire) == *seen
            })
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        drop(guard);
        let current = self.epoch.load(Ordering::Acquire);
        let changed = current != *seen;
        *seen = current;
        changed
    }
}

/// What the Python side gets back as a handle. Internally just an index.
pub struct CaptureSession {
    /// cpal stream — keep alive for the lifetime of the session.
    _stream: cpal::Stream,
    consumer: ringbuf::HeapCons<f32>,
    resampler: StreamResampler,
    metrics: Arc<Metrics>,
    target_rate: u32,
    target_dtype: OutDType,
    input_rate: u32,
    input_channels: u16,
    input_sample_format: String,
    stopped: AtomicBool,
    audio_arrival: Arc<AudioArrival>,
    arrival_seen: u64,
    /// Resampled-output staging buffer. `out_buf_start` advances without
    /// front-draining on every read; occasional compaction is amortized.
    out_buf_f32: Vec<f32>,
    out_buf_start: usize,
}
