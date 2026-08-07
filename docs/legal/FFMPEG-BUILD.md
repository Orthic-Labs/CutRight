# FFmpeg Build (LGPL-only)

## Source

- Upstream: `git.ffmpeg.org/ffmpeg.git`
- Pinned commit: `9047fa1b084f76b1b4d065af2d743df1b40dfb56`
- Pinned tag: `n8.1`
- Licence: LGPL-2.1-or-later (with the build frozen to include only LGPL
  components; no GPL, no nonfree).

## Why offline

The product may never download source at runtime. The build script
(`scripts/runtime/build-ffmpeg.py`) validates the configure flag
contract but does not fetch. The licensed source is checked out once
onto the build machine by an operator who has accepted the licence
distribution terms.

## Build contract

```text
# Forbidden flags rejected by build-ffmpeg.py --check-config:
--enable-gpl
--enable-nonfree

# Required capability probes:
ffprobe-json
h264-decode
aac-decode
libass-or-native-caption-path
zscale-or-qualified-hdr-path
```

## Target matrix

| Target           | arch    | Output path                                                           |
|------------------|---------|-----------------------------------------------------------------------|
| `host`           | auto    | `runtime/source/ffmpeg/host/auto/bin/{ffmpeg,ffprobe}`                |
| `macos-arm64`    | arm64   | `runtime/source/ffmpeg/macos-arm64/arm64/bin/{ffmpeg,ffprobe}`        |
| `linux-x86_64`   | x86_64  | `runtime/source/ffmpeg/linux-x86_64/x86_64/bin/{ffmpeg,ffprobe}`      |
| `windows-x86_64` | x86_64  | `runtime/source/ffmpeg/windows-x86_64/x86_64/bin/{ffmpeg,ffprobe}.exe` |

## Dependency licences

Every library that FFmpeg links against must be LGPL-compatible or built
with an explicit enable. The corresponding-source archive is generated
by `scripts/legal/build-corresponding-source.py --component ffmpeg
--target <target>`.

## Build steps

1. `./configure <safe flags> --enable-shared`
2. `make -j$(nproc)`
3. `make install DESTDIR=<staging>`
4. Copy the staged `bin/ffmpeg` and `bin/ffprobe` into
   `runtime/source/ffmpeg/<target>/<arch>/bin/`.
5. Run probe enumeration: `python3 scripts/runtime/build-ffmpeg.py --probe`.
6. Update `runtime/manifests/media.source.json` with measured hashes and
   run `python3 scripts/legal/build-corresponding-source.py --component
   ffmpeg --target <target>` to emit the archive.

## Acceptance

- Forbidden configure flags fail before compilation.
- Required probes are all reported by `--probe`.
- The pack archive passes tiny decode/encode/filter/mux probes.
