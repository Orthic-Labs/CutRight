# V2 — Action permissions, skill boundaries, and session write guards

Frozen by **CR-V2-B2-004**.

## 1. Least-privilege permission scopes

Eight scopes: `evidence_read`, `asset_plan`, `timeline_read`, `timeline_write`, `render`, `export`, `settings`, `pack_manage`. Each capability references exactly one permission_set.

## 2. Skill boundaries

| skill             | scopes allowed                            |
|-------------------|--------------------------------------------|
| Brand, Brand-Identity, Designer, Writing, Social | `evidence_read`, `asset_plan` |
| QA                | `evidence_read`                            |
| Editorial engine  | + `timeline_read`, `timeline_write`        |
| Render jobs       | `render`, `export`                         |
| Pack manager      | `pack_manage`, `settings`                  |

Designer/Writing/Social/QA CANNOT produce `timeline_write` actions; any attempt fails with `permission_denied` and `non_editor_skill_mutation` reason.

## 3. Session bindings

Schema: `cutright.session_binding/v1`. Every external (loopback MCP) or embedded agent session is bound to: one `project_id`, one `active_revision`, one `permission_set_id`. Cross-project writes fail with `cross_project_write_denied`.

## 4. Frontmost-project confirmation

External MCP reads from bound project are free. Writes require `frontmost_project_confirmed: true` set by Studio's "Allow external MCP write" affordance. Without it: `frontmost_project_not_confirmed`.
