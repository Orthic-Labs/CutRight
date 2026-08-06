
#[inline(always)]
fn push_mono_pcm<T>(
    data: &[T],
    in_chans: usize,
    downmix: &mut MonoDownmix,
    producer: &mut ringbuf::HeapProd<f32>,
    metrics: &Metrics,
) where
    T: Sample,
    f32: FromSample<T>,
{
    if in_chans <= 1 {
        for &s in data {
            if producer.try_push(f32::from_sample(s)).is_err() {
                metrics.record_overflow(1);
                break;
            }
        }
    } else {
        let frames = data.len() / in_chans;
        for f in 0..frames {
            let start = f * in_chans;
            let s = downmix.mix(|ch| f32::from_sample(data[start + ch]));
            if producer.try_push(s).is_err() {
                metrics.record_overflow(1);
                break;
            }
        }
        if let Some(ch) = downmix.take_pending_lock() {
            metrics.record_downmix_lock(ch);
        }
    }
}

fn encode(samples: &[f32], dtype: OutDType) -> Vec<u8> {
    match dtype {
        OutDType::Float32 => {
            let mut out = Vec::with_capacity(samples.len() * 4);
            for &s in samples {
                out.extend_from_slice(&s.to_le_bytes());
            }
            out
        }
        OutDType::Int16 => {
            let mut out = Vec::with_capacity(samples.len() * 2);
            for &s in samples {
                let clamped = s.clamp(-1.0, 1.0);
                let v = (clamped * 32767.0).round() as i16;
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_dtype_parse_accepts_known_aliases() {
        assert_eq!(OutDType::parse("int16").unwrap(), OutDType::Int16);
        assert_eq!(OutDType::parse("i16").unwrap(), OutDType::Int16);
        assert_eq!(OutDType::parse("float32").unwrap(), OutDType::Float32);
        assert_eq!(OutDType::parse("f32").unwrap(), OutDType::Float32);
        assert!(OutDType::parse("int8").is_err());
        assert_eq!(OutDType::Int16.bytes_per_sample(), 2);
        assert_eq!(OutDType::Float32.bytes_per_sample(), 4);
    }

    #[test]
    fn encode_float32_roundtrips_little_endian() {
        let samples = [0.0f32, 1.0, -1.0, 0.5];
        let bytes = encode(&samples, OutDType::Float32);
        assert_eq!(bytes.len(), samples.len() * 4);
        let decoded: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(decoded, samples);
    }

    #[test]
    fn encode_int16_clamps_and_scales() {
        // +1.0 → 32767, -1.0 → -32767, over-range clamps, 0.0 → 0.
        let bytes = encode(&[1.0, -1.0, 2.0, -2.0, 0.0], OutDType::Int16);
        let decoded: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(decoded, vec![32767, -32767, 32767, -32767, 0]);
    }

    #[test]
    fn staged_output_reads_preserve_order_without_front_drain() {
        let mut staged = (0..16_000).map(|n| n as f32).collect::<Vec<_>>();
        let mut start = 0usize;
        let mut all = Vec::new();
        while pending_resampled_len(&staged, start) > 0 {
            all.extend(take_resampled(&mut staged, &mut start, 137));
        }
        assert_eq!(all, (0..16_000).map(|n| n as f32).collect::<Vec<_>>());
        assert!(staged.is_empty());
    }

    #[test]
    fn push_mono_f32_averages_multichannel_frames() {
        let rb = HeapRb::<f32>::new(8);
        let (mut producer, mut consumer) = rb.split();
        let metrics = Metrics::new();
        let mut downmix = MonoDownmix::new(2, 48_000);

        push_mono_f32(&[1.0, -1.0, 0.5, 0.25], 2, &mut downmix, &mut producer, &metrics);

        let mut out = [0.0; 2];
        assert_eq!(consumer.pop_slice(&mut out), 2);
        assert_eq!(out, [0.0, 0.375]);
        assert_eq!(metrics.snapshot().input_overflow_count, 0);
    }

    #[test]
    fn push_mono_pcm_converts_non_f32_default_formats() {
        let rb = HeapRb::<f32>::new(8);
        let (mut producer, mut consumer) = rb.split();
        let metrics = Metrics::new();
        let mut downmix = MonoDownmix::new(2, 48_000);

        push_mono_pcm(&[1.0_f64, -1.0, 0.5, 0.25], 2, &mut downmix, &mut producer, &metrics);

        let mut out = [0.0; 2];
        assert_eq!(consumer.pop_slice(&mut out), 2);
        assert_eq!(out, [0.0, 0.375]);
        assert_eq!(metrics.snapshot().input_overflow_count, 0);
    }

    #[test]
    fn downmix_locks_onto_live_channel_when_average_cancels() {
        // Phase-inverted pair (mic + AEC reference): the average is exactly
        // zero. After one evaluation window the downmix must lock onto a real
        // channel and start emitting signal.
        let mut downmix = MonoDownmix::new(2, 1_000); // window = 500 frames
        let mut last = 0.0f32;
        for i in 0..600 {
            let v = if i % 2 == 0 { 0.5 } else { -0.5 };
            last = downmix.mix(|ch| if ch == 0 { v } else { -v });
        }
        assert_eq!(downmix.take_pending_lock(), Some(0));
        assert!(last.abs() > 0.4, "post-lock output must carry the signal, got {last}");
    }

    #[test]
    fn downmix_locks_onto_live_channel_when_others_are_dead() {
        // One live channel out of four: the average dilutes to 1/4 amplitude
        // (RMS ratio 0.25 < 0.35 floor) — must lock onto the live channel.
        let mut downmix = MonoDownmix::new(4, 1_000);
        let mut last = 0.0f32;
        for i in 0..600 {
            let v = if i % 2 == 0 { 0.2 } else { -0.2 };
            last = downmix.mix(|ch| if ch == 2 { v } else { 0.0 });
        }
        assert_eq!(downmix.take_pending_lock(), Some(2));
        assert!(last.abs() > 0.15, "post-lock output must carry the live channel, got {last}");
    }

    #[test]
    fn downmix_keeps_averaging_healthy_arrays_and_silence() {
        // Correlated stereo: average ≈ best channel, no lock.
        let mut downmix = MonoDownmix::new(2, 1_000);
        for i in 0..600 {
            let v = if i % 2 == 0 { 0.3 } else { -0.3 };
            downmix.mix(|_| v);
        }
        assert_eq!(downmix.take_pending_lock(), None);

        // Noise-floor silence must never trigger a lock even with a dead pair.
        let mut quiet = MonoDownmix::new(2, 1_000);
        for i in 0..600 {
            let v = if i % 2 == 0 { 0.001 } else { -0.001 };
            quiet.mix(|ch| if ch == 0 { v } else { -v });
        }
        assert_eq!(quiet.take_pending_lock(), None);
    }

    #[test]
    fn input_wait_distinguishes_no_callbacks_empty_callbacks_and_silent_samples() {
        assert_eq!(
            classify_input_wait(4, 4, 0),
            CaptureInputWaitStatus::NoCallbacks
        );
        assert_eq!(
            classify_input_wait(4, 5, 0),
            CaptureInputWaitStatus::NoSamples
        );
        assert_eq!(
            classify_input_wait(4, 5, 320),
            CaptureInputWaitStatus::Ready
        );
    }

    #[test]
    fn audio_arrival_wakes_without_polling() {
        let arrival = AudioArrival::default();
        let mut seen = 0;
        arrival.signal();
        assert!(arrival.wait(&mut seen, std::time::Duration::from_millis(1)));
        assert!(!arrival.wait(&mut seen, std::time::Duration::from_millis(1)));
    }

    #[test]
    fn capture_health_preserves_terminal_error_across_later_advisories() {
        let metrics = Metrics::new();
        metrics.record_async_error(CaptureErrorKind::Xrun);
        metrics.record_async_error(CaptureErrorKind::StreamInvalidated);
        metrics.record_async_error(CaptureErrorKind::DeviceChanged);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.async_error_count, 3);
        assert_eq!(
            snapshot.last_async_error_kind,
            Some(CaptureErrorKind::DeviceChanged)
        );
        assert_eq!(
            snapshot.terminal_error_kind,
            Some(CaptureErrorKind::StreamInvalidated)
        );
    }
}
