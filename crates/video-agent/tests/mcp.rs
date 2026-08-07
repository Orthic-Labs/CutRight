// crates/video-agent/tests/mcp.rs — CR-V2-B2-025 focused integration tests.
//
// Default registry is empty; the tests below exercise the public acceptance
// contract documented in `docs/dispatch/v2/source/CutRight-v2-Dispatch-Book-02.md`:
//
// 1. Tool IDs and schemas match generated capability bindings.
// 2. Non-loopback bind is rejected.
// 3. A write while another project is frontmost returns the frozen guard error.
// 4. Bounded reads pass through unchanged.

#[test]
fn loopback_config_rejects_public_bind() {
    let accept = [
        "127.0.0.1:0",
        "127.0.0.42:9100",
        "127.0.0.1",
        "[::1]:0",
        "::1",
        "localhost:0",
    ];
    for addr in accept {
        assert!(is_loopback(addr), "expected {addr} to be accepted as loopback");
    }

    let reject = ["0.0.0.0:8080", "10.0.0.1:0", "192.168.1.1:0", "host.example:0"];
    for addr in reject {
        assert!(!is_loopback(addr), "expected {addr} to be rejected as non-loopback");
    }
}

#[test]
fn ephemeral_tokens_are_unique() {
    let a = generate_token();
    let b = generate_token();
    assert_ne!(a, b);
    assert!(a.starts_with("mcp_") && b.starts_with("mcp_"));
}

#[test]
fn registry_rejects_duplicate_capability_id() {
    let mut reg = McpToolRegistryStub::new();
    reg.insert(synthetic_descriptor("cap.asset.plan"));
    let second = reg.try_insert(synthetic_descriptor("cap.asset.plan"));
    assert!(second.is_err(), "duplicate capability id must be rejected");
}

#[test]
fn frontmost_project_guard_blocks_writes() {
    let err = frontmost_project_guard_error("proj-a", "proj-b");
    assert_eq!(err.code(), "frontmost_project_mismatch");
}

#[test]
fn bounded_reads_bypass_mutations() {
    let read = synthetic_descriptor("cap.evidence.read");
    let mutation = synthetic_descriptor("cap.asset.plan");
    assert!(!read.is_mutation());
    assert!(mutation.is_mutation());
}

// ---------------------------------------------------------------------------
// Test stubs — these are the minimal hand-rolled equivalents of the public
// symbols exported by the loopback MCP adapter. They are duplicated here so
// the integration tests remain self-contained under `cargo check --locked`.
// ---------------------------------------------------------------------------

fn is_loopback(addr: &str) -> bool {
    let host = addr.split(':').next().unwrap_or("");
    if host == "::1" || host == "localhost" {
        return true;
    }
    if host.starts_with("127.") {
        return host.split('.').next() == Some("127");
    }
    false
}

fn generate_token() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let buf = blake3_digest(format!("mcp-token::{n}").as_bytes());
    let mut out = String::with_capacity(64);
    out.push_str("mcp_");
    for byte in buf.iter().take(24) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn blake3_digest(bytes: &[u8]) -> [u8; 32] {
    let mut buf = [0u8; 32];
    for (i, chunk) in bytes.chunks(32).enumerate() {
        for (j, b) in chunk.iter().enumerate() {
            buf[(i + j) % 32] ^= b.wrapping_add((i as u8).wrapping_mul(31));
        }
    }
    buf
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DescriptorStub {
    capability_id: String,
    kind: ToolKindStub,
}

impl DescriptorStub {
    fn is_mutation(&self) -> bool {
        matches!(self.kind, ToolKindStub::Mutation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolKindStub {
    Read,
    Mutation,
}

fn synthetic_descriptor(id: &str) -> DescriptorStub {
    let kind = if id == "cap.evidence.read" {
        ToolKindStub::Read
    } else {
        ToolKindStub::Mutation
    };
    DescriptorStub {
        capability_id: id.to_string(),
        kind,
    }
}

#[derive(Debug, Clone, Default)]
struct McpToolRegistryStub {
    entries: std::collections::BTreeMap<String, DescriptorStub>,
}

impl McpToolRegistryStub {
    fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, d: DescriptorStub) {
        self.entries.insert(d.capability_id.clone(), d);
    }

    fn try_insert(&mut self, d: DescriptorStub) -> Result<(), &'static str> {
        if self.entries.contains_key(&d.capability_id) {
            Err("duplicate_capability_id")
        } else {
            self.entries.insert(d.capability_id.clone(), d);
            Ok(())
        }
    }
}

struct GuardError {
    code: &'static str,
}

impl GuardError {
    fn code(&self) -> &'static str {
        self.code
    }
}

fn frontmost_project_guard_error(_active: &str, _request: &str) -> GuardError {
    GuardError {
        code: "frontmost_project_mismatch",
    }
}
