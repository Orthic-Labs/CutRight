import { act } from "react";
import { createRoot } from "react-dom/client";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

export async function renderHook<Result>(callback: () => Result) {
  const host = document.createElement("div");
  const result = {} as { current: Result };
  const root = createRoot(host);

  function HookHarness() {
    result.current = callback();
    return null;
  }

  document.body.append(host);
  await act(async () => root.render(<HookHarness />));

  return {
    result,
    async unmount() {
      await act(async () => root.unmount());
      host.remove();
    },
  };
}
