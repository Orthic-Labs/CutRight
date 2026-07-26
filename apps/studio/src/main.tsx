import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type Project = { project_id: string; kind: string; review_mode: string; outputs: { id: string; aspect: string; width: number; height: number }[] };
const demo: Project = { project_id: "qa-demo", kind: "mixed_creator_content", review_mode: "reviewed", outputs: [{ id: "youtube", aspect: "16:9", width: 1920, height: 1080 }, { id: "reels", aspect: "9:16", width: 1080, height: 1920 }] };

function App() {
  const qa = new URLSearchParams(location.search).has("qa");
  const [path, setPath] = useState("");
  const [project, setProject] = useState<Project | null>(qa ? demo : null);
  const [error, setError] = useState("");
  async function load() {
    setError("");
    try { setProject(await invoke<Project>("read_project", { path })); }
    catch (reason) { setProject(null); setError(String(reason)); }
  }
  return <main className="app" aria-label="CutRight Studio">
    <header><div><p className="eyebrow">LOCAL VIDEO REVIEW</p><h1>CutRight Studio</h1></div><span className="status">{qa ? "QA mode" : "Local-only"}</span></header>
    <section className="loader" aria-label="Open project"><label htmlFor="project-path">Project folder</label><div><input id="project-path" value={path} onChange={(event) => setPath(event.target.value)} placeholder="/path/to/MyVideo.video-project" /><button onClick={load}>Open project</button></div>{error && <p role="alert">{error}</p>}</section>
    {project ? <section className="workspace"><article><p className="eyebrow">PROJECT</p><h2>{project.project_id}</h2><p>{project.kind.replaceAll("_", " ")} · {project.review_mode}</p></article><article><p className="eyebrow">DELIVERABLES</p><ul>{project.outputs.map((output) => <li key={output.id}><strong>{output.id}</strong><span>{output.width}×{output.height} · {output.aspect}</span></li>)}</ul></article><article><p className="eyebrow">REVIEW GATES</p><p>Evidence, transcription benchmark, captions, and final QA are reviewed from the project artifact directory.</p></article></section> : <section className="empty"><h2>Open a CutRight project</h2><p>Studio reads project JSON and stays a review surface; `videoctl` remains the media engine.</p></section>}
  </main>;
}
createRoot(document.getElementById("root")!).render(<React.StrictMode><App /></React.StrictMode>);
