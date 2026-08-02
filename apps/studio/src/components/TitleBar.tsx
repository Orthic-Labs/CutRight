// Top app titlebar: wordmark, project title, theme toggle, help. Extracted
// out of App.tsx (REV2 audit decomposition) — pure move, no behavior
// change.
export function TitleBar({
  title,
  theme,
  onToggleTheme,
  onHelp,
}: {
  title: string;
  theme: string;
  onToggleTheme: () => void;
  onHelp: () => void;
}) {
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="wordmark">
        Cut<span>Right</span>
      </div>
      <div className="project-title">
        {title}
        <small>.video-project</small>
      </div>
      <div className="title-actions">
        <button aria-label="Toggle theme" onClick={onToggleTheme}>
          {theme === "dark" ? "☼" : "◐"}
        </button>
        <button aria-label="Keyboard shortcuts" onClick={onHelp}>
          ?
        </button>
      </div>
    </header>
  );
}
