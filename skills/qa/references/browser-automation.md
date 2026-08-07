# Browser Automation — Real Browser Rule (CutRight v2)

For ANY task that requires opening a URL, clicking an element, scraping a page, or screenshotting a website, route to one of these engines in this order:

## Tier 1 — Bundled local CDP runners (default in CutRight v2)

The vendored QA runners (`skills/qa/scripts/`) drive an already-installed Chrome/Edge through
headless flags and raw CDP. They execute as typed capabilities inside the signed qa runtime pack
(`cutright://capability/qa.qa`, `qa.qa_functional`, `qa.qa_shot`). No browser download, no
Playwright, no Puppeteer, and no network dependency beyond the local app under test.

## Tier 2 — Host-native browser tool (optional host capability)

When the host environment provides a native browser tool, it may be used for local app QA,
screenshots, navigation, click-type flows, and visual checks. It is an optional host capability —
never a required path in a base CutRight image.

## Tier 3 — Heavy snapshot tools as last resort

Upstream allowed heavy snapshot tooling as a last resort when a specific tool is available and its
screenshot/snapshot behavior is genuinely needed. Avoid for iterative workflows when it returns
large page snapshots every action.

## Excluded in CutRight v2

- Standalone machine-specific browser daemons (the upstream host CLI and its global browser
  binaries) are not part of the qa closure. Upstream carried machine-specific daemon-repair steps
  for them; those steps are not vendored.
- Logged-in session automation and account access are excluded: QA observes the local app under
  test only.

Borrow harness discipline from upstream:
- Save successful site-specific flows as reusable domain notes instead of rediscovering selectors.
- Create small helper code only when repeated browser work needs it.
- Keep helper files editable and reviewable.
- Do not enable broad auto-learning/helper-writing on financial, medical, identity, or
  account-management pages without the operator's approval.
