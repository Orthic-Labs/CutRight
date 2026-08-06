//! heardright_capture — low-latency audio capture (cpal + rubato).
//!
//! The cpal capture/ring-buffer/resample/metrics logic lives in `capture`,
//! `device`, `resample`, `metrics`. Linked as an `rlib` by the Rust sidecar
//! with `default-features = false`.

pub mod capture;
pub mod device;
pub mod metrics;
pub mod resample;

pub use capture::{CaptureFirstBuffer, CaptureSession, OutDType, SessionConfig};
pub use device::{
    list_input_devices, resolve_device, CaptureFormFactor, CaptureTransport, DeviceInfo,
};
pub use metrics::{CaptureErrorKind, MetricsSnapshot};
pub use resample::StreamResampler;
