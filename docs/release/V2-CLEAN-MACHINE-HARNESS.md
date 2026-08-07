# V2 Clean-Machine Harness

The clean-machine harness verifies that the v2 offline bundle runs
end-to-end on a fresh, supported target with no developer toolchain
and no outbound network. It is the authoritative acceptance proof
for "no external dependencies".

## Preconditions

The harness is run on a target that satisfies the following:

* Fresh OS user; no `~/.cargo`, no `~/.npm`, no `~/.local`, no
  `~/.venvs`.
* `PATH` empty except for `/usr/bin:/bin`.
* Outbound network denied by policy (host firewall or routing rule),
  not just by application preference.
* No Python, Node, FFmpeg, Ollama, HeardRight, CodeRight or workspace
  skill present on `PATH`.
* The offline bundle root and the workspace's `samples/v2` directory
  are present at known paths.

## Postcondition

The harness writes a result file conforming to
`schemas/release/clean-machine-result.schema.v1.json`. The file
records five checks; each check must pass for the overall result
to be a pass.

| Check                          | Pass condition                                               |
| ------------------------------ | ------------------------------------------------------------ |
| `no_external_tool_on_path`     | None of the eight forbidden binaries is on `PATH`.            |
| `network_deny`                 | DNS resolution for the probe hosts fails by policy.           |
| `offline_bundle_markers`       | `app/`, `packs/`, `licences/`, `checksums/`, `signatures/` exist. |
| `all_four_samples_present`     | Each of the four `samples/v2` projects ships its manifest.   |
| `four_lanes_accepted`          | The set of sample lanes equals `{creator, speech, creative, vision}`. |

## Running

```bash
python3 scripts/qa/v2-clean-machine/run.py \
    --target host \
    --bundle release/v2/staging \
    --result release/v2/clean-machine-host.json
```

The harness never opens a TCP connection, never uploads, never
publishes. It only reads local paths, runs `shutil.which` and
`socket.getaddrinfo`, and writes a JSON result.

## Acceptance

* The result file passes JSON-schema validation against
  `clean-machine-result.schema.v1.json`.
* `overall_passed` is `true`.
* The fixture in `fixtures/schemas/release/clean-machine-result/v1/valid/basic.json`
  matches the schema.
* `network_attempts` is zero and `external_runtime_dependencies` is
  zero in every accepted result.
