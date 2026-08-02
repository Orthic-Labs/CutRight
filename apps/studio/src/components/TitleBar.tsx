import { RegisterSwitch } from "./RegisterSwitch";
import type { Register } from "../types";

// Top app titlebar: wordmark, project title, QA-only register switcher,
// theme toggle, help. Extracted out of App.tsx (REV2 audit decomposition)
// — pure move, no behavior change; the register switcher is new (redesign
// spec Phase 2) and only mounts when `qa` is true.
export function TitleBar({
  title,
  theme,
  onToggleTheme,
  onHelp,
  qa,
  register,
  setRegister,
}: {
  title: string;
  theme: string;
  onToggleTheme: () => void;
  onHelp: () => void;
  qa: boolean;
  register: Register;
  setRegister: (value: Register) => void;
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
      {qa && <RegisterSwitch register={register} setRegister={setRegister} />}
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
