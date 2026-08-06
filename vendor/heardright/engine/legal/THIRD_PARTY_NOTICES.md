# HeardRight Third-Party Notices

**Notice version 1.0 · Effective July 16, 2026**  
**Product:** HeardRight 0.1.108 for Windows and macOS<br>
**Publisher:** Damned Ventures LLC

Build provenance: HeardRight 0.1.108; supported targets `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, and `x86_64-apple-darwin`.<br>
inputs: `pnpm-lock.yaml` sha256=`3947ee15f7cd25eddc9881c4dc0ad2e13dc3b2bf7bd485e20ac2889891cca750`; `src-tauri/Cargo.lock` sha256=`da6ba917089b549acbb362575b9808b04f9a3c07f3541fe5fe8f60ce0fa96a0a`; `heardright-engine/Cargo.lock` sha256=`e3212ee21e3a8cbc450473afd2eb35f588f6128568bae0a5f70d0ef4cfe0a23c`.<br>
generated-by=`@rightkit/legal` notice contract 1 plus `cargo-about 0.9.1` for the shell and sidecar closures and `pnpm licenses list --prod`.

HeardRight includes or can download third-party software, models, fonts, and data. Those components are licensed under their own terms, not the Right Suite Desktop License. Nothing in the HeardRight EULA restricts rights granted by an applicable third-party license.

## Machine-generated dependency appendices

- `THIRD_PARTY_RUST_LICENSES.html` — target-filtered Rust dependency names, versions, license selections, and full license texts; sha256=`0dc531989df38634b596c02435b0adfe99d569650ed5c8e13246a967147640fb`.
- `THIRD_PARTY_NPM_LICENSES.html` — production npm package names, versions, declared licenses, and packaged license/notice files; sha256=`410995d484052b387eedf8ca50476feee77296fcaca63eed7f9828fce9a60124`.

Internal `@rightkit/*` packages and private `heardright_*` crates are Damned Ventures LLC proprietary components and are intentionally excluded from the third-party appendices.

## Speech and wake models

### NVIDIA Parakeet Unified English 0.6B

Parakeet Unified is retired and is no longer offered to new installs. An upgraded install may temporarily continue using its existing local copy until Parakeet TDT is verified on disk, after which HeardRight switches models and removes the retired copy. Unified is derived from `nvidia/parakeet-unified-en-0.6b`, licensed under the NVIDIA Open Model License Agreement. It may be converted, optimized, or quantized for the supported runtime.

**Required NVIDIA Notice:** Licensed by NVIDIA Corporation under the NVIDIA Open Model License

A copy of the NVIDIA Open Model License dated October 24, 2025 is bundled at `third-party/NVIDIA_OPEN_MODEL_LICENSE_2025-10-24.pdf`. The authoritative model page is <https://huggingface.co/nvidia/parakeet-unified-en-0.6b>. Use is also subject to the NVIDIA Trustworthy AI terms linked by that license.

### NVIDIA Parakeet TDT 0.6B v3

HeardRight's current default local speech model is derived from `nvidia/parakeet-tdt-0.6b-v3`, © NVIDIA Corporation, licensed under Creative Commons Attribution 4.0 International. HeardRight's runtime copies may be converted, optimized, or quantized. Source: <https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3>. License: <https://creativecommons.org/licenses/by/4.0/>. The legal code is bundled at `third-party/CC-BY-4.0.txt`.

### OpenAI Whisper and related runtimes

Optional multilingual transcription can use OpenAI Whisper large-v3-turbo weights under the MIT License. Platform builds can use WhisperKit/Argmax components on macOS and whisper.cpp on Windows. The relevant MIT texts are bundled at:

- `third-party/WHISPER_MIT.txt`
- `third-party/WHISPERKIT_MIT.txt`
- `third-party/WHISPER_CPP_MIT.txt`

### HeardRight wake model

The HeardRight wake model was trained by Damned Ventures LLC using WavLM-derived tooling or representations. Microsoft WavLM/UniLM source is MIT-licensed; its notice is bundled at `third-party/WAVLM_MIT.txt`. HeardRight-specific weights, training data, and conversion work remain proprietary to the extent permitted by the upstream license.

### Sherpa-ONNX keyword controls

Realtime Zephyr command detection uses Sherpa-ONNX 1.13.4 with `sherpa-onnx-kws-zipformer-zh-en-3M-2025-12-20`. Sherpa-ONNX source, runtime & accompanying model payload are supplied under Apache License 2.0. License text is bundled with payload at `kws/LICENSE-SHERPA-ONNX.txt`. Source: <https://github.com/k2-fsa/sherpa-onnx>.

## Native runtimes and storage

- ONNX Runtime — MIT License, © Microsoft Corporation; `third-party/ONNX_RUNTIME_MIT.txt`.
- DirectML redistributable — Microsoft redistributable terms supplied with `Microsoft.AI.DirectML`; `third-party/DIRECTML_LICENSE.txt`.
- SQLCipher — BSD-style license; `third-party/SQLCIPHER_LICENSE.txt`.
- OpenSSL — Apache License 2.0 for current OpenSSL 3 releases. The Rust appendix contains the selected license text for the shipped dependency closure.
- Tauri, React, Rust crates, and npm packages — the exact target/production appendices above.

## Fonts

- Hanken Grotesk and Spline Sans Mono — SIL Open Font License 1.1; bundled at `third-party/HANKEN_GROTESK_OFL.txt` and `third-party/SPLINE_SANS_MONO_OFL.txt`.
- Tanker — Indian Type Foundry Free Font License 1.0. Fontshare grants free personal and commercial use including apps; the retrieved license page is bundled at `third-party/FONTSHARE_ITF_FFL.html` and remains available at <https://www.fontshare.com/licenses/itf-ffl>. The font software is used unmodified and is not separately offered for download.

Third-party names and marks identify their respective components and do not imply endorsement. If an accompanying file conflicts with this summary, the accompanying third-party license controls.
