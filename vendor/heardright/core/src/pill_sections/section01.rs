// Pill — pure parts: the `PillEvent` payload (serde), the work-area clamp
// geometry, and the legacy message→event mapping. The Tauri window show/hide,
// positioning, and monitor queries live in `src-tauri/src/pill.rs`.

use serde::{Deserialize, Serialize};

/// Pill control-surface states (LOCKED spec — HR_PILL_DESIGN_PROPOSAL_2026-05-25.md;
/// visual signed off via `scripts/pill_states_mockup.html`). The pill is an
/// icon/timer surface plus short transient status text for completed actions.
/// `Error.label` is for the tooltip/aria only, never rendered in the pill.
/// `Voice` is a separate
/// high-frequency amplitude update that drives the recording glow WITHOUT
/// changing the rendered mode (so it doesn't reset the timer). Serializes
/// `{ "kind": "voice", "level": 0.5 }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PillEvent {
    Hide,
    /// Resting sliver (copper). Emitted at rest (Armed) and at startup — the
    /// pill is an always-on overlay (Adrian sign-off 2026-05-28). Hovering the
    /// sliver is the start affordance.
    Idle,
    /// Immediate acknowledgement while the audio stream is resuming. The
    /// renderer promotes this to `Recording` only after the first mic samples.
    Starting,
    /// Recording: reactive bars + timer. Renderer ticks the timer locally.
    Recording,
    /// Mic amplitude 0.0..=1.0 for the contained voice glow. Updates the glow
    /// only; does not change mode. (Fallback: renderer breathes gently if no
    /// Voice events arrive — amplitude wiring is Phase 3 Chunk C.)
    Voice {
        level: f32,
    },
    /// Copper breath. `cancellable=false` for final-only ASR after stop, so the
    /// renderer shows no controls on hover.
    Processing {
        cancellable: bool,
    },
    /// Legacy generic completion event. Shared React renderers downgrade this
    /// to a text status ("Done"); new producers should prefer `Status`.
    Success,
    /// Short transient non-interactive status, e.g. `Pasted`, `Sent`, `Saved`, `Copied`.
    Status {
        label: String,
    },
    /// Pulsing red x. `label` is tooltip/aria text only — never shown inline.
    Error {
        label: String,
    },
}

/// Clamp a physical (x, y) so a `pill_w`×`pill_h` window sits fully inside
/// `[work_left, work_right] × [work_top, work_bottom]`. The `.max(lo)` on each
/// upper bound guards the degenerate case where the pill is larger than the
/// work area (a zero-sized / anomalous monitor report would otherwise make
/// `clamp` panic on lo > hi). Pure i32 math — also exercised by
/// `tests/pill_clamp_tests.rs` via the `src-tauri` re-export.
pub fn clamp_into_work(
    x: i32,
    y: i32,
    pill_w: i32,
    pill_h: i32,
    work_left: i32,
    work_top: i32,
    work_right: i32,
    work_bottom: i32,
) -> (i32, i32) {
    let max_x = (work_right - pill_w).max(work_left);
    let max_y = (work_bottom - pill_h).max(work_top);
    (x.clamp(work_left, max_x), y.clamp(work_top, max_y))
}

/// Hover follows the rendered pill capsule/action dock exactly on macOS, where
/// the native window may include transparent slack. On Windows the pill window
/// is kept pill-only, so the stable HWND rect is the hover target.
pub const PILL_HOVER_HITBOX_MARGIN: f64 = 0.0;
pub const PILL_HOVER_IDLE_POLL_MS: u64 = 90;
pub const PILL_HOVER_ACTIVE_POLL_MS: u64 = 40;
pub const PILL_HOVER_ENTER_TICKS: u32 = 1;
pub const PILL_HOVER_LEAVE_TICKS: u32 = 3;
pub const PILL_CURSOR_EMIT_MIN_DELTA: f64 = 1.0;

pub fn pill_should_be_visible(
    active: bool,
    always_show_resting: bool,
    fullscreen: bool,
) -> bool {
    !fullscreen && (active || always_show_resting)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PillHoverGate {
    inside_ticks: u32,
    outside_ticks: u32,
    hovering: bool,
}

impl PillHoverGate {
    pub fn update(&mut self, inside: bool) -> bool {
        if inside {
            self.inside_ticks = self.inside_ticks.saturating_add(1);
            self.outside_ticks = 0;
        } else {
            self.outside_ticks = self.outside_ticks.saturating_add(1);
            self.inside_ticks = 0;
        }
        self.hovering = if self.hovering {
            self.outside_ticks < PILL_HOVER_LEAVE_TICKS
        } else {
            inside && self.inside_ticks >= PILL_HOVER_ENTER_TICKS
        };
        self.hovering
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Hit-test a cursor point in WebView client coordinates against the actual
/// rendered pill capsule/action-dock rectangle reported by the frontend. The
/// overlay window can be taller than the pill when a popup is visible; hover
/// expansion must follow this hitbox, not the whole transparent window.
pub fn pill_hover_hitbox_contains(
    hitbox_x: f64,
    hitbox_y: f64,
    hitbox_width: f64,
    hitbox_height: f64,
    client_x: f64,
    client_y: f64,
) -> bool {
    if !hitbox_x.is_finite()
        || !hitbox_y.is_finite()
        || !hitbox_width.is_finite()
        || !hitbox_height.is_finite()
        || !client_x.is_finite()
        || !client_y.is_finite()
    {
        return false;
    }
    if hitbox_width < 1.0 || hitbox_height < 1.0 {
        return false;
    }
    let margin = PILL_HOVER_HITBOX_MARGIN;
    client_x >= hitbox_x - margin
        && client_x <= hitbox_x + hitbox_width + margin
        && client_y >= hitbox_y - margin
        && client_y <= hitbox_y + hitbox_height + margin
}

pub fn windows_hover_gate_input(inside_window: bool) -> bool {
    inside_window
}

/// Legacy message → event mapping for the static `show_pill(&str)` API.
/// Reduced to the v2 states (no text inside the pill). Unknown/empty → Idle/Hide.
pub fn event_for_message(message: &str) -> PillEvent {
    let lower = message.to_ascii_lowercase();
    if lower.contains("listening") || lower.contains("recording") {
        PillEvent::Recording
    } else if lower.contains("transcribing")
        || lower.contains("thinking")
        || lower.contains("pasting")
    {
        PillEvent::Processing { cancellable: false }
    } else if lower.contains("error") {
        PillEvent::Error {
            label: message.to_string(),
        }
    } else if lower.is_empty() {
        PillEvent::Hide
    } else {
        PillEvent::Idle
    }
}

// ---------------------------------------------------------------------------
// Pure layout (P1 native pill) — sizes/geometry only, NO render deps. The binary
// (`src-tauri/src/pill/native.rs`) measures text via ab_glyph and passes widths in; it draws
// (and, in P2, hit-tests) at the rects this module computes — single source of truth, unit-tested
// here without a window. From scripts/pill_states_mockup.html v3.
// ---------------------------------------------------------------------------

/// Render mode (distinct from `PillEvent`, which drives it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillState {
    Idle,
    Recording,
    Processing,
    Success,
    Error,
}

/// Map an incoming event to a render mode. `Hide` → None (hide the pill); `Voice` → None
/// (updates the glow level only — does NOT change the mode, so it never resets the timer).
pub fn state_for_event(ev: &PillEvent) -> Option<PillState> {
    match ev {
        PillEvent::Idle => Some(PillState::Idle),
        PillEvent::Starting => Some(PillState::Processing),
        PillEvent::Recording => Some(PillState::Recording),
        PillEvent::Processing { .. } => Some(PillState::Processing),
        PillEvent::Success | PillEvent::Status { .. } => Some(PillState::Success),
        PillEvent::Error { .. } => Some(PillState::Error),
        PillEvent::Hide | PillEvent::Voice { .. } => None,
    }
}

/// Logical-px layout constants (× scale → device px). From the v3 mockup.
pub mod dims {
    pub const IDLE_W: f32 = 96.0;
    pub const IDLE_H: f32 = 18.0;
    pub const IDLE_HOVER_H: f32 = 32.0;
    pub const REC_W: f32 = 102.0;
    pub const REC_H: f32 = 28.0;
    pub const PROC_W: f32 = 66.0;
    pub const PROC_H: f32 = 30.0;
    pub const HOVER_SCALE: f32 = 1.05;
    pub const RESULT_W: f32 = 42.0; // success / error glyph capsule in live PillOverlay.tsx
    pub const RESULT_H: f32 = 30.0;
    pub const SUCCESS_HOVER_W: f32 = 116.0; // pad11 + check20 + gap9 + copy28 + gap9 + list28 + pad11
    pub const ERROR_HOVER_W: f32 = 46.0; // React transformed error raster captures as 92px at 2x.
    pub const ERROR_HOVER_H: f32 = 32.0;
    pub const HOVER_H: f32 = 44.0; // recording hover height
    pub const PAD_X: f32 = 11.0; // hover padding-x
    pub const CORE_GAP: f32 = 9.0;
    pub const SIDE_GAP: f32 = 7.0;
    pub const ICON_BOX: f32 = 28.0;
    pub const GLYPH_BOX: f32 = 20.0;
    pub const DOT: f32 = 8.0;
    /// Reactive level-bar cluster width: 5 bars × 3 + 4 gaps × 2.5 (PillOverlay.tsx).
    pub const BARS_W: f32 = 25.0;
}

/// Visual constants mirrored from `src/pill/PillOverlay.tsx` so native renderers
/// can stay pixel-aligned with the React pill.
pub mod react_design {
    pub const EMBER: (u8, u8, u8) = (0xff, 0x56, 0x30);
    pub const COFFEE: (u8, u8, u8) = (0x21, 0x1d, 0x1a);
    pub const FG: (u8, u8, u8) = (0xf3, 0xee, 0xea);
    pub const OK: (u8, u8, u8) = (0x3f, 0xd0, 0x6a);
    pub const ERR: (u8, u8, u8) = (0xe5, 0x67, 0x5b);
    pub const EDGE_ALPHA: f32 = 0.28;
    pub const EMBER_REST_ALPHA: f32 = 0.35;
    pub const SUCCESS_HOLD_MS: u64 = 1100;
    pub const ERROR_HOLD_MS: u64 = 1700;
    pub const PROCESSING_HOLD_MS: u64 = 9000;

    const BAR_WEIGHTS: [f32; 5] = [0.58, 0.86, 1.0, 0.8, 0.52];
    const BAR_MIN: f32 = 3.0;
    const BAR_MAX: f32 = 22.0;
    const NOISE_GATE: f32 = 0.09;

    pub fn bar_height(level: f32, i: usize) -> f32 {
        let g = ((level.max(0.0) - NOISE_GATE) / (1.0 - NOISE_GATE)).max(0.0);
        let gained = g.min(1.0).powf(0.85);
        let weight = BAR_WEIGHTS.get(i).copied().unwrap_or(1.0);
        (BAR_MIN + gained * weight * (BAR_MAX - BAR_MIN)).round()
    }
}

/// Recording-hover width (DEVICE px), computed from visible children + gaps + padding — NOT a
/// constant. Layout L→R: pad · bars · gap · timer · gap · cancel · gap · stop · sidegap · send · pad
/// (matches PillOverlay.tsx recording-hover). `timer_w` is the measured timer width in DEVICE px.
pub fn rec_hover_w(scale: f32, timer_w: f32) -> f32 {
    use dims::*;
    // fixed (logical) = pad + bars + gap + gap + icon + gap + icon + sidegap + icon + pad = 165
    let fixed = PAD_X
        + BARS_W
        + CORE_GAP
        + CORE_GAP
        + ICON_BOX
        + CORE_GAP
        + ICON_BOX
        + SIDE_GAP
        + ICON_BOX
        + PAD_X;
    fixed * scale.max(0.0) + timer_w.max(0.0)
}

/// Pill size in DEVICE px for `(state, hover)` at `scale`. `timer_w` (device px) is used only by
/// recording-hover (the auto-width state). Degenerate/negative scale → 0-area, never panics.
/// P1 renders non-hover only; hover sizes exist for P2 and are tested here.
pub fn geom(state: PillState, hover: bool, scale: f32, timer_w: f32) -> (f32, f32) {
    use dims::*;
    let s = scale.max(0.0);
    match (state, hover) {
        (PillState::Idle, false) => (IDLE_W * s, IDLE_H * s),
        (PillState::Idle, true) => (IDLE_W * s, IDLE_HOVER_H * s),
        (PillState::Recording, false) => (REC_W * s, REC_H * s),
        (PillState::Recording, true) => (rec_hover_w(s, timer_w), HOVER_H * s),
        (PillState::Processing, false) => (PROC_W * s, PROC_H * s),
        (PillState::Processing, true) => (PROC_W * HOVER_SCALE * s, PROC_H * HOVER_SCALE * s),
        (PillState::Success, false) | (PillState::Error, false) => (RESULT_W * s, RESULT_H * s),
        (PillState::Success, true) => (SUCCESS_HOVER_W * s, RESULT_H * s),
        (PillState::Error, true) => (ERROR_HOVER_W * s, ERROR_HOVER_H * s),
    }
}

/// A clickable icon region in DEVICE px, relative to the pill's top-left (same space
/// the renderer draws icons in). The renderer draws at these rects AND the Win32 layer
/// hit-tests against them — one layout pass, so draw-rect == hit-rect, no drift (P2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// Half-open containment: `[x, x+w) × [y, y+h)`.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// What clicking a pill icon does. The Win32 backend maps each to the matching
/// app action on the main thread.
/// Mapping is from the v3 mockup's `title=` attributes (the "copy exactly" source).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillAction {
    Start,
    Cancel,
    Stop,
    StopAndSend,
    CopyLast,
    Repaste,
    History,
}

/// Clickable hit-targets for `(state, hover)` in DEVICE px relative to the pill's
/// top-left, matching the renderer's icon walk exactly. `timer_w` (device px) only
/// shifts the recording-hover right cluster. Interaction is hover-gated: non-hover and
/// processing return none (status-only). Idle-hover is the whole pill (the mockup's
/// "same width, taller target"). Centre status indicators (bars/timer, check, x) are not clickable.
pub fn hit_targets(
    state: PillState,
    hover: bool,
    scale: f32,
    timer_w: f32,
) -> Vec<(PillAction, Rect)> {
    use dims::*;
    if !hover {
        return Vec::new();
    }
    let s = scale.max(0.0);
    let (w, h) = geom(state, hover, s, timer_w);
    let ib = ICON_BOX * s;
    let iy = (h - ib) * 0.5;
    let pad = PAD_X * s;
    let gap = CORE_GAP * s;
    let sg = SIDE_GAP * s;
    let tw = timer_w.max(0.0);
    let icon = |x: f32| Rect {
        x,
        y: iy,
        w: ib,
        h: ib,
    };
    match state {
        // L→R: centered [start] · gap · [history], matching the React hover actions.
        PillState::Idle => {
            let x_start = (w - (ib + gap + ib)) * 0.5;
            vec![
                (PillAction::Start, icon(x_start)),
                (PillAction::History, icon(x_start + ib + gap)),
            ]
        }
        // L→R: pad · bars · gap · timer · gap · [cancel] · gap · [stop] · sidegap · [send] · pad
        PillState::Recording => {
            let bars = BARS_W * s;
            let x_cancel = pad + bars + gap + tw + gap;
            let x_stop = x_cancel + ib + gap;
            let x_send = x_stop + ib + sg;
            vec![
                (PillAction::Cancel, icon(x_cancel)),
                (PillAction::Stop, icon(x_stop)),
                (PillAction::StopAndSend, icon(x_send)),
            ]
        }
        // L→R: pad · status(check) · gap · [copy] · gap · [history] · pad.
        PillState::Success => vec![
            (PillAction::CopyLast, icon(pad + 20.0 * s + gap)),
            (PillAction::History, icon(pad + 20.0 * s + gap + ib + gap)),
        ],
        // React keeps error hover as a scaled compact status glyph, with no actions.
        PillState::Error => Vec::new(),
        // Transcribing: status-only, nothing actionable.
        PillState::Processing => Vec::new(),
    }
}

/// Hit-test a click at `(px, py)` (DEVICE px, pill-relative) → the action under it.
pub fn hit_test(
    state: PillState,
    hover: bool,
    scale: f32,
    timer_w: f32,
    px: f32,
    py: f32,
) -> Option<PillAction> {
    hit_targets(state, hover, scale, timer_w)
        .into_iter()
        .find(|(_, r)| r.contains(px, py))
        .map(|(a, _)| a)
}
