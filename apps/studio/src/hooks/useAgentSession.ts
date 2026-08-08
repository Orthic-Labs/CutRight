import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../lib/api";
import type { AgentEvent, AgentProvider, AgentRoute, AgentSessionContract } from "../contracts/agent";

interface AgentSnapshot extends AgentSessionContract { session_id: string; status: NonNullable<AgentSessionContract["status"]>; events: readonly AgentEvent[]; }

export function useAgentSession(projectPath: string) {
  const [routes, setRoutes] = useState<AgentRoute[]>([]);
  const [session, setSession] = useState<AgentSnapshot | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const sessionRef = useRef<string | null>(null);
  const refreshRoutes = useCallback(async () => { try { setRoutes(await call<AgentRoute[]>("agent_routes", { path: projectPath })); } catch (reason) { setError(String(reason)); setRoutes([]); } }, [projectPath]);
  const replay = useCallback(async (sessionId = sessionRef.current) => { if (!sessionId) return; try { const next = await call<AgentSnapshot>("agent_session_events", { session_id: sessionId }); sessionRef.current = next.session_id; setSession(next); } catch (reason) { setError(String(reason)); } }, []);
  useEffect(() => { void refreshRoutes(); }, [refreshRoutes]);
  useEffect(() => { if (!sessionRef.current) return; const timer = window.setInterval(() => void replay(), 1500); return () => window.clearInterval(timer); }, [replay]);
  const start = useCallback(async (goal: string, provider: AgentProvider) => { setBusy(true); setError(""); try { const next = await call<AgentSnapshot>("agent_session_start", { path: projectPath, goal, provider }); sessionRef.current = next.session_id; setSession(next); return next; } catch (reason) { setError(String(reason)); return null; } finally { setBusy(false); } }, [projectPath]);
  const command = useCallback(async (name: "agent_session_pause" | "agent_session_resume" | "agent_session_cancel") => { if (!sessionRef.current) return; setBusy(true); setError(""); try { await call(name, { session_id: sessionRef.current }); await replay(); } catch (reason) { setError(String(reason)); } finally { setBusy(false); } }, [replay]);
  return { routes, session, error, busy, start, pause: () => command("agent_session_pause"), resume: () => command("agent_session_resume"), cancel: () => command("agent_session_cancel"), replay, refreshRoutes };
}
