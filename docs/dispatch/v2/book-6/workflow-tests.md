# Studio workflow tests (CR-V2-B6-025)

Drives create/import → Make Versions → Story/Beats review → correction → Design/Motion audition → compare → QA → final selection across the four lanes (recorded_footage, repurpose, explainer, anchored_creative).
One workflow is replayed through the embedded agent and one through optional MCP, asserting persisted actions are equivalent.
Restart during a job and during an uncommitted UI selection: no committed work is lost and no source is corrupted.
