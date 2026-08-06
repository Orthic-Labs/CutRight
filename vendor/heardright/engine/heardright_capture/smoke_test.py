"""Smoke test for heardright_capture.

Lists devices, captures ~2 seconds of audio at 16k mono int16, dumps to WAV,
and prints RMS + metrics. Run inside the heardright .venv-build.
"""
import math
import struct
import sys
import time
import wave
from pathlib import Path

import heardright_capture as hc


def main() -> int:
    print(f"heardright_capture v{hc.__version__}")
    devices = hc.list_devices()
    print(f"\n{len(devices)} input devices:")
    for d in devices:
        marker = " [DEFAULT]" if d["is_default"] else ""
        print(
            f"  id={d['id']:>2}  rate={d['native_rate']:>6}  ch={d['channels']}"
            f"  {d['name']}{marker}"
        )
    if not devices:
        print("ERROR: no input devices")
        return 1

    target_rate = 16000
    target_dtype = "int16"
    duration_s = 2.0

    print(f"\nStarting capture (default device, {target_rate} Hz mono {target_dtype})...")
    handle = hc.start(
        device_id=None,
        target_rate=target_rate,
        target_channels=1,
        target_dtype=target_dtype,
        block_ms=20,
    )
    info = hc.session_info(handle)
    print(f"session_info: {info}")

    chunks: list[bytes] = []
    bytes_per_sample = 2
    target_bytes = int(target_rate * duration_s) * bytes_per_sample
    collected = 0
    t0 = time.time()
    while collected < target_bytes:
        # Pull ~100 ms at a time, blocking up to 250 ms
        chunk = hc.read_blocking(handle, min_samples=int(target_rate * 0.1), timeout_ms=250)
        if chunk:
            chunks.append(chunk)
            collected += len(chunk)
        if time.time() - t0 > duration_s + 1.0:
            break
    elapsed = time.time() - t0

    metrics = hc.stop(handle)
    audio = b"".join(chunks)
    n_samples = len(audio) // bytes_per_sample
    print(f"\nCaptured {n_samples} samples ({len(audio)} bytes) in {elapsed:.2f}s")

    # Compute RMS
    if n_samples > 0:
        sum_sq = 0.0
        for i in range(n_samples):
            s = struct.unpack_from("<h", audio, i * 2)[0]
            sum_sq += (s / 32768.0) ** 2
        rms = math.sqrt(sum_sq / n_samples)
        rms_db = 20.0 * math.log10(rms) if rms > 0 else float("-inf")
        print(f"RMS = {rms:.6f}  ({rms_db:.1f} dBFS)")
    else:
        print("WARNING: no samples captured")

    # Write WAV
    out_path = Path(__file__).parent / "smoke_test.wav"
    with wave.open(str(out_path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(target_rate)
        wf.writeframes(audio)
    print(f"WAV written to {out_path}  ({out_path.stat().st_size} bytes)")

    print("\nMetrics:")
    for k, v in metrics.items():
        print(f"  {k}: {v}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
