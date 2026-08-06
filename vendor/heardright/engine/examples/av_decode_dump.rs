//! Measurement only — decode files through the AVFoundation path and write the
//! resulting 16 kHz mono PCM as WAV, so the same corpus can be transcribed once
//! per decoder and the WER compared. `av_decode::decode_to_16k_mono` is the pure
//! AVFoundation path (file_transcribe's wrapper falls back to ffmpeg, which would
//! defeat the comparison).
//!
//! Usage: av_decode_dump <out_dir> <file> [<file> ...]

use std::path::Path;

use heardright_engine::av_decode;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: av_decode_dump <out_dir> <file> [<file> ...]");
        std::process::exit(2);
    }
    let out_dir = Path::new(&args[0]);
    std::fs::create_dir_all(out_dir).expect("create out dir");

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    for input in &args[1..] {
        let path = Path::new(input);
        match av_decode::decode_to_16k_mono(path) {
            Ok(samples) => {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let out = out_dir.join(format!("{name}.wav"));
                let mut w = hound::WavWriter::create(&out, spec).expect("create wav");
                for s in &samples {
                    let clamped = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    w.write_sample(clamped).expect("write sample");
                }
                w.finalize().expect("finalize");
                println!("av  {name}: {} samples", samples.len());
            }
            Err(e) => eprintln!("av  {input}: DECLINED ({e})"),
        }
    }
}
