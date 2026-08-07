# v2 user documentation

This directory contains the offline v2 product documentation. It is
shipped in the offline bundle and is reachable from the Studio `Help`
menu without an internet connection.

The documentation matches the v2 capability registry and the v2
acceptance matrix. When the registry changes, the docs in this
directory must be regenerated to match.

## Pages

| Page                                     | What it covers                                  |
| ---------------------------------------- | ----------------------------------------------- |
| [first-launch.md](first-launch.md)       | First launch, packs and Make Versions            |
| [make-versions.md](make-versions.md)     | Producing a version from a project               |
| [review-and-correction.md](review-and-correction.md) | Reviewing decisions and correcting outputs |
| [design-and-motion.md](design-and-motion.md) | Native render, design and motion tools       |
| [qa-and-export.md](qa-and-export.md)     | QA floors and export pipeline                   |
| [recovery.md](recovery.md)               | Recovery, repair, rollback and migration         |
| [privacy.md](privacy.md)                 | Local logs, telemetry-off defaults, data flow   |
| [packs.md](packs.md)                     | Local pack set and how to verify/activate        |

## External tools

The documentation never instructs you to install Python, Node,
FFmpeg, Ollama, HeardRight or any third-party runtime. If a step
appears to require an external tool, file a documentation bug against
this directory; the dispatcher enforces the rule.
