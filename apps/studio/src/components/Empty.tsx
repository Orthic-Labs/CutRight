export function Empty({ onOpen, error }: { onOpen: () => void; error: string }) {
  return (
    <main className="empty-project">
      <div className="wordmark">
        Cut<span>Right</span>
      </div>
      <p>Open a local video project to review its evidence and cuts.</p>
      <button onClick={onOpen}>Open project…</button>
      <code>videoctl project init My.video-project</code>
      {error && <p role="alert">{error}</p>}
    </main>
  );
}
