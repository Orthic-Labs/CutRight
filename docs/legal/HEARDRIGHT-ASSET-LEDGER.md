# HeardRight Asset Ledger

Status: Book 1 (CR-V2-B1-013). Machine-readable form:
`imports/v2/heardright-assets.json` (validated by
`tools/import-closure/validate_asset_ledger.py`). Vendored source graph:
`imports/v2/graphs/heardright-source.json` (HeardRight pin
`b60bff947f12ffa9d25e94ad27e8ff30db006a24`).

Rule: no model byte was copied into the repository by this task. Every
referenced runtime asset resolves only from signed CutRight runtime-pack
provenance, never from the vendored tree. Unverifiable licences are
`blocked_unresolved` and are release-blocking per
`docs/legal/V2-IMPORT-POLICY.md` §3. No licence is inferred from a
filename or from repository ownership.

## 1. In-tree asset (vendored, hash-bound)

| Asset | Path | sha256 (prefix) | Class | Status | Pack |
| --- | --- | --- | --- | --- | --- |
| whisper mel filterbank | `vendor/heardright/engine/src/whisper_mel_128.bin` | `2a5f9822…` (full hash in ledger JSON) | dataset | `audited_separately` | speech |

The mel filterbank (128×201 f32 LE, `include_bytes!` in
`whisper_coreml_sections/section02.rs`) is generated from the openai/whisper
mel script; the upstream MIT notice is preserved at
`vendor/heardright/engine/legal/third-party/WHISPER_MIT.txt`. A pack-manifest
row must close it before it enters the signed speech pack.

## 2. Referenced runtime assets (bytes NOT vendored)

Destination pack for every adopted row is **`speech`**.

| asset_id | Source | Licence status | Redistribution |
| --- | --- | --- | --- |
| `parakeet-tdt-primary` | NVIDIA Parakeet TDT 0.6B family (Hugging Face `nvidia/parakeet-tdt-*`); upstream revision unverified; loaded as CoreML bundle `parakeet-tdt-v3` and quantized ONNX layout | `blocked_unresolved` | none — NVIDIA Open Model License text exists only as HeardRight's vendored copy (`engine/legal/third-party/NVIDIA_OPEN_MODEL_LICENSE_2025-10-24.pdf`), not a CutRight grant |
| `parakeet-rnnt-unified` | NVIDIA Parakeet RNNT 0.6B family (Hugging Face `nvidia/parakeet-rnnt-0.6b`); upstream revision unverified; loaded as CoreML bundle `parakeet-unified-en-0.6b` | `blocked_unresolved` | none — same evidence gap |
| `silero-vad-onnx-16k` | Silero VAD exact upstream ONNX bytes, https://github.com/snakers4/silero-vad (corpus row `silero-vad`, pin `76e3dc408eb2a5c655c34e230d2d5459b4439daa`); HeardRight names an opset-15 variant `silero_vad_16k_op15.onnx` whose byte identity is unverified | `licensed_for_redistribution` (MIT per corpus row) | permitted by MIT; exact byte SHA-256 frozen by the pack builder from the pinned silero-vad source |
| `silero-vad-coreml-16k` | HeardRight-generated CoreML conversion `silero_vad_16k.mlmodelc` | `blocked_unresolved` | none — conversion provenance/output bytes unverified |
| `whisper-large-v3-turbo-coreml` | openai/whisper large-v3-turbo converted by HeardRight to `AudioEncoder.mlmodelc` + `TextDecoder.mlmodelc` under `coreml/whisper-multi` | `blocked_unresolved` | none — source weights MIT (WHISPER_MIT.txt) but conversion bytes unverified |
| `whisper-tokenizer-json` | `tokenizer.json` bundled beside the CoreML bundle | `blocked_unresolved` | none — byte identity unverified |
| `whisper-win-ggml-bin` | whisper.cpp GGML quantized model (`whisper_turbo_q5_k.bin` / `*.gguf` under `whisper-win/`) | `blocked_unresolved` | none — third-party quantization with no recorded licence row |

## 3. Excluded rows

| asset_id | Reason |
| --- | --- |
| `sherpa-kws-wakeword-transducer` | Wake-word keyword-spotting transducer (`encoder/decoder/joiner-epoch-13-avg-2-chunk-16-left-64*.onnx` + `tokens.txt`); wake-word lane not adopted by CutRight v2 — excluded by dispatch. |
| `kws-owner-manifests` | Wake-word manifests inside the excluded `tauri-app-next/src-tauri` app tree. |

## 4. Closure requirements before the speech pack may be signed

1. Fetch exact upstream bytes for every `blocked_unresolved` row, record
   their SHA-256, and attach a licence row that is actually granted to
   CutRight (never inferred).
2. Freeze the exact Silero VAD ONNX bytes from the pinned `silero-vad`
   corpus source and drop the unverified opset-15 naming.
3. Close the `audited_separately` mel-filterbank row in the pack manifest.
4. The release validator must reject the release while any row above is
   unresolved (policy §3).
