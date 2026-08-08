import { call } from "./lib/api";

export type RationalTime = { numerator: number; denominator: number };
export type NativePlayerFrame = { x: number; y: number; width: number; height: number };
export type ScopedBookmark = { token: number; path: string; stale: boolean; refreshedBookmark?: string | null };
export type NativePlayer = {
  id: number;
  load(path: string, scopeToken: number): Promise<void>;
  seek(time: RationalTime): Promise<void>;
  play(): Promise<void>;
  pause(): Promise<void>;
  attach(frame: NativePlayerFrame): Promise<void>;
  resize(frame: NativePlayerFrame): Promise<void>;
  detach(): Promise<void>;
  setRate(rate: number): Promise<void>;
  setVolume(volume: number): Promise<void>;
  currentTime(): Promise<number>;
  duration(): Promise<number>;
  destroy(): Promise<void>;
};

/** Gated adapter only; HTML media remains default transport. */
export async function createNativePlayer(): Promise<NativePlayer> {
  const id = await call<number>("native_player_create");
  return {
    id,
    load: (path, scopeToken) => call("native_player_load", { id, path, scopeToken }),
    seek: ({ numerator, denominator }) => call("native_player_seek", { id, numerator, denominator }),
    play: () => call("native_player_play", { id }),
    pause: () => call("native_player_pause", { id }),
    attach: (frame) => call("native_player_attach", { id, frame }),
    resize: (frame) => call("native_player_resize", { id, frame }),
    detach: () => call("native_player_detach", { id }),
    setRate: (rate) => call("native_player_set_rate", { id, rate }),
    setVolume: (volume) => call("native_player_set_volume", { id, volume }),
    currentTime: () => call("native_player_current_time", { id }),
    duration: () => call("native_player_duration", { id }),
    destroy: () => call("native_player_destroy", { id }),
  };
}

export const createSecurityScopedBookmark = (path: string) => call<string>("create_security_scoped_bookmark", { path });
export const resolveSecurityScopedBookmark = (bookmark: string) => call<ScopedBookmark>("resolve_security_scoped_bookmark", { bookmark });
export const releaseSecurityScopedBookmark = (token: number) => call<void>("release_security_scoped_bookmark", { token });
