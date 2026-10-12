# Directive proposals for 0.14.51 — `.new` + diff, NOTHING APPLIED

These are **proposals**, not pack files. They are kept here so the change is reviewable in the repo
while the live directives stay untouched (directives are constitution class: the deliverable of a
directive change is `<file>.new` plus a diff, and the live file is never edited in place).

★**Do not copy a `.new` next to the live directive.** `cysjavis-pack/bin/cys-dept` accepts
`<MASTER_DIRECTIVE.md>.new` as a superset reference when it promotes a CEO template
(`ceo_template_superset_ok`, cys-dept:1484-1499, called at :1583-1587), so a `.new` placed beside the
live file would become live input to that path. Approval first, then the editing seat applies it.

★**`APPLY-FIRST-S6.md` is mandatory reading before applying the MASTER/CEO proposal**: alone it reddens eight
named content pins (one root cause), and the companion re-pin is a code change that this deliverable does not carry.
`CSO_DIRECTIVE.md.new` (S2) **is** applicable on its own — measured, zero new red.

| file | item | gate |
|---|---|---|
| `CSO_DIRECTIVE.md.new` (+ `.diff`) | S2 — how the quiescing label may be read | PASSED the what-ships gate: S2 ships (commit 937248ab, visibility only) |
| `MASTER_DIRECTIVE.md.new` (+ `.diff`) | the owner's 2026-10-12 reversal, **authority only** | the S6 MECHANISM does NOT ship in 0.14.51; this text claims no behaviour |
| `CEO_TEMPLATE.md.new` (+ `.diff`) | derived | generated from the MASTER proposal by `scripts/gen_ceo_template.py` (byte concatenation) |

S5 has no text: its code landed but its carrier is never called in product code, so a directive sentence about its relay would promise a path nothing takes.
`select_carrier` is defined in `src/bin/cysd/s5_escalation.rs` and its only callers are that file's own unit tests, all of them after the `cfg(test)` boundary.

Measurement log for all of this: `~/Desktop/CYSjavis/_evidence/dept5-directive-new-20261012/`
(WHAT-SHIPS.md, REQUIREMENTS-TRACE.md, REFLECT-1-adversarial.md, REFLECT-2-architect.md, raw/).
