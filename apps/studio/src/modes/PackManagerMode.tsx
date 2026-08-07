import { useState } from "react";

export type PackAction = "verify" | "repair_from_payload" | "activate" | "rollback";

export interface PackDescriptor {
  id: string;
  active: boolean;
  compatible: string[];
  signature_valid: boolean;
  size: number;
  source: "local_verified_bundle" | "remote" | "unknown";
  available: boolean;
  measured_target_status: "supported" | "unsupported";
}

export interface PackActionResult {
  ok: boolean;
  message: string;
  active_after: string | null;
}

/**
 * PackManager — list, verify, repair, activate and rollback local payload
 * packs. Never offers a web download and never accepts an unverified source.
 */
export function PackManager({ packs }: { packs: PackDescriptor[] }) {
  const [busy, setBusy] = useState<PackAction | null>(null);
  const [active, setActive] = useState<string | null>(
    packs.find((p) => p.active)?.id ?? null
  );
  const [lastMessage, setLastMessage] = useState<string>("");

  function runPackAction(p: PackDescriptor, action: PackAction) {
    if (p.source !== "local_verified_bundle") {
      setLastMessage(
        "pack source is not a locally verified bundle; offline v2 refuses to proceed"
      );
      return;
    }
    setBusy(action);
    // The actual command is dispatched to Tauri by the harness; here we
    // only update the view state.
    if (action === "rollback") {
      setActive((prev) => (prev === p.id ? null : prev));
    } else if (action === "activate") {
      setActive(p.id);
    }
    setLastMessage(`${action} ${p.id}`);
    setBusy(null);
  }

  return (
    <section data-testid="pack-manager">
      <header>
        <h2>Packs</h2>
        <span data-active={active}>{active ?? "<none>"}</span>
      </header>
      <ul>
        {packs.map((p) => (
          <li key={p.id} data-pack-id={p.id}>
            {p.id} — {p.compatible.join(",")} — sig {String(p.signature_valid)}
            {p.source !== "local_verified_bundle" && (
              <em data-testid="unverified-source">unverified source</em>
            )}
            <button
              type="button"
              data-action="verify"
              onClick={() => runPackAction(p, "verify")}
              disabled={busy !== null}
            >
              verify
            </button>
            <button
              type="button"
              data-action="repair_from_payload"
              onClick={() => runPackAction(p, "repair_from_payload")}
              disabled={busy !== null || !p.available}
            >
              repair from payload
            </button>
            <button
              type="button"
              data-action="activate"
              onClick={() => runPackAction(p, "activate")}
              disabled={busy !== null || !p.signature_valid}
            >
              activate
            </button>
            <button
              type="button"
              data-action="rollback"
              onClick={() => runPackAction(p, "rollback")}
              disabled={busy !== null}
            >
              rollback
            </button>
          </li>
        ))}
      </ul>
      {lastMessage && <p role="status">{lastMessage}</p>}
    </section>
  );
}

export default PackManager;
