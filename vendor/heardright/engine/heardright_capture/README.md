# heardright_capture

Low-latency audio capture for Heard Right, written in Rust (cpal + rubato). The
production Rust worker links the crate directly; an optional PyO3 / maturin
binding supports legacy Python consumers.

The current Tauri sidecar uses this crate as its production capture path.

## What it does

- Opens an input device via cpal (WASAPI on Windows, ALSA on Linux, CoreAudio on macOS).
- Runs a minimal-discipline audio callback: convert sample, downmix to mono, push into a lock-free SPSC ring buffer, update atomic counters. No allocation, no logging, no Python touched in the callback.
- Resamples on the consumer side using a stateful rubato `SincFixedIn`.
- Returns either little-endian `int16` or `float32` bytes, ready to feed an ASR pipeline.

## Python API

```python
import heardright_capture as hc

devices = hc.list_devices()
# [{'id': 0, 'name': 'Microphone (...)' , 'native_rate': 48000, 'channels': 2, 'is_default': True}, ...]

handle = hc.start(
    device_id=None,        # None = system default; or int from list_devices
    target_rate=16000,
    target_channels=1,     # only mono is supported right now
    target_dtype="int16",  # or "float32"
    block_ms=20,           # advisory; current build uses cpal's default block size
)

info = hc.session_info(handle)
# {'input_rate': 48000, 'input_channels': 2, 'target_rate': 16000, 'target_dtype': 'int16'}

# Non-blocking pull (returns empty bytes if nothing buffered yet).
chunk = hc.read(handle, max_samples=16000)

# Block until at least N samples are available, with a timeout.
chunk = hc.read_blocking(handle, min_samples=16000, timeout_ms=500)

# Mid-session metrics snapshot.
m = hc.metrics(handle)

# Stop and release device. Returns final metrics.
final = hc.stop(handle)
```

## Build

For the production Rust path:

```bash
cargo test --all-targets --no-default-features
cargo run --example live_capture_smoke --no-default-features
```

For the optional Python extension, install Python 3.11+ with `maturin`:

```bash
# from the crate root
maturin build --release
```

The wheel lands in `target/wheels/`. Install it into the Heard Right venv:

```bash
pip install target/wheels/heardright_capture-*.whl --force-reinstall
```

For dev iteration, `maturin develop --release` builds and installs in one step (must be run inside an active venv).

### Windows

Build with the venv on PATH:

```cmd
D:\Claude\heardright\.venv-build\Scripts\maturin.exe build --release
```

No extra cpal feature flags are needed — the default WASAPI backend is correct.

### Linux / macOS

Same `maturin build --release` from the crate root. On Linux you need ALSA dev headers (`libasound2-dev` on Debian/Ubuntu).

## Logging

Set the `HEARDRIGHT_LOG` env var before importing the module to control verbosity:

```bash
export HEARDRIGHT_LOG=info  # or debug, trace, warn, error
```

Logs go to stderr.

## Threading & safety notes

- The cpal callback runs on the OS audio thread. It only does atomic ops + a single `push_slice` (or per-sample `try_push` when downmixing). No mutexes, no allocations.
- The production worker creates, uses, pauses, resumes, and drops
  `CaptureSession` on its owning thread. No unsafe `Send` implementation is
  required.
- The optional Python session registry (`HashMap<u64, CaptureSession>`) is
  behind a `Mutex`; the audio thread never touches it.
- All Python-facing entrypoints release the GIL via `py.allow_threads` before any blocking work, so a blocking `read_blocking` call won't stall other Python threads.
