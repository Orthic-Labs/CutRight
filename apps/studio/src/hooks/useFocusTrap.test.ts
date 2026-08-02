import { afterEach, describe, expect, it } from "vitest";
import { getFocusableElements, trapTabKey } from "./useFocusTrap";

// jsdom's querySelector uses `getElementById` as a fast path for `#id`
// selectors and only checks whether the (first, document-wide) match is a
// descendant of the scoping element — so leftover containers from earlier
// tests with the same ids shadow the current test's elements. Clear the
// DOM between tests instead of hunting for unique ids everywhere.
afterEach(() => {
  document.body.innerHTML = "";
});

function buildDialog(): HTMLElement {
  const container = document.createElement("div");
  container.innerHTML = `
    <button id="close">x</button>
    <button id="mid">mid</button>
    <button id="last" disabled>last (disabled)</button>
    <a id="link" href="#">link</a>
  `;
  document.body.appendChild(container);
  return container;
}

describe("getFocusableElements", () => {
  it("lists focusable elements in DOM order and skips disabled ones", () => {
    const container = buildDialog();
    const ids = getFocusableElements(container).map((el) => el.id);
    expect(ids).toEqual(["close", "mid", "link"]);
  });

  it("returns an empty list for a container with nothing focusable", () => {
    const container = document.createElement("div");
    container.innerHTML = "<span>no controls here</span>";
    document.body.appendChild(container);
    expect(getFocusableElements(container)).toEqual([]);
  });
});

describe("trapTabKey", () => {
  it("wraps Tab on the last element back to the first", () => {
    const container = buildDialog();
    const last = container.querySelector<HTMLElement>("#link")!;
    last.focus();
    const event = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true });
    trapTabKey(event, container);
    expect(event.defaultPrevented).toBe(true);
    expect(document.activeElement?.id).toBe("close");
  });

  it("wraps Shift+Tab on the first element back to the last", () => {
    const container = buildDialog();
    const first = container.querySelector<HTMLElement>("#close")!;
    first.focus();
    const event = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true, shiftKey: true });
    trapTabKey(event, container);
    expect(event.defaultPrevented).toBe(true);
    expect(document.activeElement?.id).toBe("link");
  });

  it("does not interfere with Tab between interior elements", () => {
    const container = buildDialog();
    const mid = container.querySelector<HTMLElement>("#mid")!;
    mid.focus();
    const event = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true });
    trapTabKey(event, container);
    expect(event.defaultPrevented).toBe(false);
  });

  it("ignores non-Tab keys", () => {
    const container = buildDialog();
    const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    trapTabKey(event, container);
    expect(event.defaultPrevented).toBe(false);
  });

  it("traps focus inside an empty container by preventing default", () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const event = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true });
    trapTabKey(event, container);
    expect(event.defaultPrevented).toBe(true);
  });
});
