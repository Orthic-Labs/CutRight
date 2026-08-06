# Sherpa KWS native patch

Upstream is sherpa-onnx v1.13.4 at `142807252687d81b40d6315f23470a1512a00de3`.
Apply `patches/0001-expire-active-keyword-hypotheses.patch` before building; SHA-256: `f8700cbb7efbab01363c3c4f901a5300fe21d53d8d4e307c435d2c1d2bd1a707`.

`kMaxKeywordActiveFrames = 38` at 40 ms/frame retains every active keyword context for 1.52 s. In-place `RemoveIf` erases only expired paths; blank/root is added only when beam is empty. Production phrases complete within 1.12 s.

The patch also removes upstream's private trailing-blank reset from
`DecodeStreams`. HeardRight's VAD/reset generation is sole stream-reset owner,
so native timestamps cannot silently restart while outer sample origin remains
unchanged.

Mac target: Release shared C API, `SHERPA_ONNX_ENABLE_C_API=ON`, `CMAKE_OSX_ARCHITECTURES=arm64;x86_64`, `CMAKE_OSX_DEPLOYMENT_TARGET=14.0`.
Output: `libsherpa-onnx-c-api.dylib`, universal `arm64` + `x86_64`, SHA-256 `1f16f3dc70afaa25e774f36113365f7ace836c927ea485350124ac3f3d836ecf`.

Windows DLL still contains prior TTL-only patch. Rebuild it with this complete
patch before calling Windows fixed.
