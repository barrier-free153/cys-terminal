# ★APPLY CONDITIONS for `MASTER_DIRECTIVE.md.new` / `CEO_TEMPLATE.md.new` (the S6 authority-only proposal)
Written by worker-2 on the dept-5 head's order, 2026-10-12. Read this BEFORE applying either file.

## ★THIS `.new` IS NOT APPLIED WITHOUT ITS COMPANION RE-PIN
Applying it alone **reddens eight existing content pins**. I measured this by running the pins against the
`.new` and against the live tree and attributing the difference by name
(`cysjavis-pack/bin/tests/test_cso_directive_rev.py`, 73 tests: live **9** failures → with this `.new` **17**;
raw/pins-fail-names-LIVE.txt vs raw/pins-fail-names-NEW.txt):

1. `test_safety_clauses_are_present_verbatim` (directive=`MASTER_DIRECTIVE.md`)
2. `test_safety_clauses_are_present_verbatim` (directive=`CEO_TEMPLATE.md`)
3. `test_negative_control_catches_each_restored_clause` (`MASTER_DIRECTIVE.md`, label `queue.starved 사람 운영자 전용 복원`)
4. `test_negative_control_catches_each_restored_clause` (`CEO_TEMPLATE.md`, same label)
5. `test_remedy_token_and_llm_prohibition_match_the_code` (`MASTER_DIRECTIVE.md`)
6. `test_remedy_token_and_llm_prohibition_match_the_code` (`CEO_TEMPLATE.md`)
7. `test_negative_safety_clause_controls`
8. `test_wp6_clause_declares_tool_dormancy` (`MASTER_DIRECTIVE.md`)

**One root cause**, not eight: the removed verbatim clause is read by the shared helper `missing_safety_clauses()`.
The pins' own docstring states the required response, and it is not "delete the pin":
> 조항이 바뀌면 이 검체가 붉어지는 것이 정상이며, 그때의 지시는 '핀을 지워라' 가 아니라 **'바뀐 뜻을 의식적으로 재핀하라'** 다.
That re-pin is a **code** change, so it is deliberately absent from this deliverable (this ticket applies nothing).
Whoever applies the `.new` lands the re-pin **in the same commit**.

## ★`CSO_DIRECTIVE.md.new` (S2) IS APPLICABLE ON ITS OWN
Measured separately with MASTER/CEO restored to live bytes: **9 failures = the live control's 9, zero new red**
(raw/pins-S2-only.txt). S2's behaviour also ships (commit 937248ab), so its sentence promises nothing absent.

## What else travels with the S6 text
- `CEO_TEMPLATE.md.new` is **generated**, not hand-written: template = `scripts/ceo_template_header.md` + separator +
  `MASTER_DIRECTIVE.md` bytes. Applying the MASTER proposal **requires** running `python3 scripts/gen_ceo_template.py`;
  `--check` is rc 1 while the template is stale (measured: 108,541 ≠ 107,422) and rc 0 after regeneration.
- `governance.rs:8561 REMEDY_LLM_SUFFIX` stays as it is — the head ruled on 2026-10-12 that changing it now would be
  product text for an absent mechanism. The `.new` names that tail and says it is cut together with this sentence in the
  release that ships the mechanism.

## ★Why these conditions are in a sibling file and not inside the `.new` bytes
A `.new` file is the **proposed file content**. Anything written at its head would be applied into the constitution, and
because `CEO_TEMPLATE.md` is a **byte concatenation** of the header fragment and the MASTER bytes, it would also be
copied into the generated template — the proposal would pollute what it proposes. So the conditions sit beside the
files, are named in the folder's `README.md`, and are reported to the head. If the head wants them inside the bytes
instead, it is a one-line move and I will do it on his word.
