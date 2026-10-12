//! S5 — the queue-block warning leaves the owner's sight. The carrier that would take a person-remedy
//! block to a person is DEFINED in this file, and no code outside this file calls it. A wide `grep -rn
//! s5_escalation src/` mixes together things of different kinds, so the four
//! facts are separated here, each saying what was counted (measured at this head):
//!  1. **The module is declared and compiled into the binary** — `main.rs:52` holds `mod s5_escalation;`.
//!     This file is built; it is not excluded from the binary.
//!  2. **Lines outside this file that USE the `s5_escalation::` path: 0** (counted: `grep -rn` hits for
//!     that path in `src/`, excluding this file and comment lines).
//!  3. **Callers outside this file: 0** for `select_carrier`, `IncidentStore`, `record_unanswered`,
//!     `may_reapproach` and `Incident::owner_text` (counted: `grep -rn` hits in `src/` outside this
//!     file). Superficial hits belong to other code: `owner_text` also names a different function in
//!     `teamtoken.rs`, and `eligible(` matches `alert_eligible`, `external_eligible` and
//!     `quarantine_eligible`. What exercises these items is this file's own unit tests.
//!  4. ★**The name also appears INSIDE this file, and no occurrence here is a use**: the sentences of
//!     this header, and the self source pin `include_str!("s5_escalation.rs")` at the
//!     bottom of this file. ★**This header states no total for the wide grep, on purpose.** Excluding
//!     this header, the hits are **2** — the declaration in `main.rs` and this file's own
//!     `include_str!` pin — and that figure is stated as self-excluding because it is one. A total
//!     that counts the header too counts a corpus its own text belongs to, so the next edit of the
//!     header falsifies it: the sentence that used to say the grep "returns 3 hits" was already wrong
//!     when it was written, because the same commit's new header lines raised the real count to 6.
//!     A reader who runs the wide grep must be able to tell which hit is
//!     which — otherwise a true statement reads as a lie. ★The rule for this is already written at
//!     the pin itself, near the bottom of this file: "a source pin whose own needles appear in its own source matches
//!     itself and can only ever fail". This header once said the name "appears NOWHERE in src/", which
//!     its own text refuted — the same defect the pin's comment warns about, in the same file.
//!
//! ## IMPLEMENTATION CONTRACT (read this before changing anything below)
//!
//! A queue block that **only a person can clear** must still reach a person after the owner's rule removed
//! queue-block *warnings* from his sight. One escalation, two carriers, chosen by **one ordered** selector:
//!
//! * **Carrier A** — the master seat reports through its ordinary chain (directive text; nothing in this file calls it).
//! * **Carrier B** — the master seat is itself blocked by a person-remedy code (or its hang test is met), so
//!   the actor that would carry the report is the failing one. The escalation WOULD leave through the CSO's
//!   pre-existing owner-feed escape; no code calls [`select_carrier`] yet. No new feed kind and no new owner surface is added.
//!
//! ### THE ORDER IS PART OF THE CONTRACT, NOT AN IMPLEMENTATION DETAIL (minor m3)
//! Carrier A's literal predicate is `!eligible` and carrier B's is `eligible OR hang`, so **a calm-code hang
//! satisfies both**. [`select_carrier`] is therefore **ORDERED: B is tried first, A otherwise**, and the same
//! order is stated here in words so the text cannot drift from the code.
//!
//! ### CARRIER B'S LIFECYCLE WHEN THE OWNER DOES NOT ANSWER (O-7, head declaration 3)
//! * **The wait is BOUNDED and its expiry is TERMINAL AND NON-AUTHORISING.** The bound is the caller's
//!   requested wait — **120 s by default, 600 s maximum** (`handlers.rs` `feed.push` clamps
//!   `unwrap_or(120).clamp(1, 600)`) — measured in **ACTIVE time**: the countdown decrements only while the
//!   daemon is not paused (`main.rs`, the 1-second tick), so **an owner pause does not consume the wait** and
//!   a kill-switch does not drop waiting workers into a timeout-deny.
//! * **A reply that arrives after expiry authorises NOTHING, and the product already enforces that**:
//!   `feed.reply` refuses any item whose status is not `pending` with `invalid_params` / "item already
//!   resolved", and the daemon's timeout path only rewrites an item that is still `pending`, returning the
//!   real decision when a reply won the race. This module therefore **adds no guard**; it records the outcome.
//! * **The incident is not lost (once the store is called).** Expiry WOULD be recorded durably against the **stable incident id** as the
//!   outcome [`OUTCOME_UNANSWERED`] with the attempt count. Silence toward the owner must not become silence
//!   in the record.
//! * **A re-approach is a NEW ITEM, never a revival** — no writer can modify a settled item — and it happens
//!   only when the ordered selector fires again on a **FRESH OBSERVATION** of the same incident id, **never on
//!   a timer alone**. The attempt count travels in the text the owner sees, so the owner reads *"third time"*
//!   and not an identical card.
//! * **It is capped**: at most one re-approach per cooldown and at most `K` per 24 h, reusing the parameters
//!   O-6 already names. **When the cap is reached the incident DOWNGRADES to carrier A in the explicit state
//!   `unanswered × K` — it does not disappear.**
//!
//! ### THE PARAMETERS ARE CHOSEN AND NOT MEASURED (O-6)
//! [`CAP_PER_24H`] (K = 6), [`CAP_WINDOW_SECS`] (24 h) and [`REAPPROACH_COOLDOWN_SECS`] are **CHOSEN, NOT
//! MEASURED**. They are reminder/ceiling values, not evidence of a measured flood threshold. Changing one is a
//! design change, not a tuning change.
//!
//! ### THIS RECORD REMEMBERS; IT DOES NOT GATE
//! Nothing here refuses, blocks, holds or gates anything. A failed write to this record must never prevent a
//! feed append, a settle, or a delivery — the record's failure is returned to its caller and reported, and the
//! escalation proceeds. A record that can stall the queue would trade the defect this design fixes for a worse
//! one.
//!
//! ### THE STORE IS APPEND-ONLY AND TOLERANT PER LINE
//! [`IncidentStore`] appends one JSON object per line and **never rewrites or truncates the file**. On read, an
//! unparsable line is **preserved and skipped** — it can never cause the other records to be dropped. This is
//! deliberate: the product's own feed restore reads its file with an all-or-nothing UTF-8 read inside a branch
//! that encloses the parse loop and the compaction, so one bad byte there loses every pending item. This file
//! must not be able to do that to itself.

#![allow(dead_code)] // The mechanism and its contract land with this module; the relay that calls it is
                     // directive text owned by a separate work unit, so the entry points are not yet wired
                     // into the tick. Declared rather than hidden: see the S5 implementation report.

use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------------------------------
// O-1 — THE ONE COMPILED LIST. The eligibility set and the S6 exclusion live HERE, together, so the two
// tables cannot drift apart silently.
// ---------------------------------------------------------------------------------------------------

/// What a remedy code means for the escalation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeClass {
    /// Only a person can clear it ⇒ the escalation exists for exactly these.
    Person,
    /// It clears itself or is the owner's own act ⇒ never escalated.
    Calm,
}

/// Every code in `governance::QUEUE_REMEDY_CODES`, classified **exactly once**.
/// The person half is the same 11 the app's own reader treats as person-held (`ui/src/starvednotice.ts`
/// `STARVED_HUMAN_CODES`); the calm half is the 3 that clear themselves. The disjointness test below is what
/// keeps this list and those two tables from diverging.
pub(crate) const QUEUE_CODE_CLASS: [(&str, CodeClass); 14] = [
    ("paused", CodeClass::Calm),
    ("approval", CodeClass::Calm),
    ("wait", CodeClass::Calm),
    ("machine_residue", CodeClass::Person),
    ("phantom_count", CodeClass::Person),
    ("after_cursor_text", CodeClass::Person),
    ("human_draft", CodeClass::Person),
    ("input_pending_unknown", CodeClass::Person),
    ("answer_modal", CodeClass::Person),
    ("alt_screen", CodeClass::Person),
    ("empty_seat", CodeClass::Person),
    ("prompt_unknown", CodeClass::Person),
    ("stale_screen", CodeClass::Person),
    ("unknown", CodeClass::Person),
];

/// ★THE S6 EXCLUSION IS EMPTY **BECAUSE S6'S MECHANISM DOES NOT SHIP IN 0.14.51** — so for this release
/// eligibility = the whole person set is CORRECT, not merely fail-closed. It is a stated gap for the
/// release that does ship the mechanism, never a claim that S6 clears nothing.
///
/// S5 must not escalate a code that S6's resolver clears by itself. Three facts, each with where it came from:
///
/// 1. **S6's mechanism does not ship in this release.** `docs/proposals/directives-0.14.51/README.md` states
///    that the MASTER proposal carries the owner's reversal as *authority only* and that "the S6 MECHANISM
///    does NOT ship in 0.14.51; this text claims no behaviour". So for 0.14.51 there is nothing automatic to
///    exclude.
/// 2. **No clearable-code table exists to copy.** Read under a narrow, named authorisation from the S6 design
///    folder: `DESIGN-S6.md:119-122` gives a three-way split (machine-fixable = `alt_screen`,
///    `prompt_unknown`, *conditional on the product not having already tried*, with `empty_seat` deferred;
///    human-text-held; no-op), while `DESIGN-S6-r2.md:383-386` replaces the operative concept with an **ASK**
///    set of three classes and takes `phantom_count`'s ASK away, and `DESIGN-S6-r3.md:322` keeps r2 unchanged.
///    **Those are different kinds of table, not three statements of one table**, and the nearest candidate is
///    conditional on a per-row runtime property rather than on the code. A table three documents state in
///    three vocabularies is not a measurement, so it is not copied into a compiled list here.
/// 3. **Therefore eligibility is the whole person set for 0.14.51**, and that is correct rather than
///    merely conservative: fact 1 means there is nothing automatic to exclude in this release. This
///    column stays empty *with its reason attached* — an unlabelled empty column would assert "S6
///    clears nothing" as a property of the resolver, which is not what was measured. The
///    S6-subset invariant stays ARMED in the disjointness test for the release that ships the
///    mechanism.
///
/// When it is filled, it must stay this shape: each entry records **which document and line** it came from,
/// because a table read from a design is a DESIGN FACT and must never be presented as a source measurement.
pub(crate) const S6_CLEARABLE: [&str; 0] = [];

/// ELIGIBLE = the person class, minus anything S6's resolver is defined to clear.
///
/// Fail-closed on purpose: a code that is **not** in the 14 is eligible (it is filed as `unknown`, a person
/// class), because an unrecognised code is exactly the case where a person is most likely needed. `None`
/// means the seat is not blocked or its queue is empty — not eligible, and no special case is needed, because
/// the code is non-null whenever the raw reason is.
pub(crate) fn eligible(code: Option<&str>) -> bool {
    let Some(code) = code else { return false };
    if S6_CLEARABLE.contains(&code) {
        return false;
    }
    match QUEUE_CODE_CLASS.iter().find(|(c, _)| *c == code) {
        Some((_, CodeClass::Person)) => true,
        Some((_, CodeClass::Calm)) => false,
        None => true, // fail-closed: an unknown code is a person's problem
    }
}

// ---------------------------------------------------------------------------------------------------
// O-5 + m3 — THE ORDERED SELECTOR
// ---------------------------------------------------------------------------------------------------

/// Who carries the escalation. The order of the variants is the order of the test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Carrier {
    /// The CSO's pre-existing owner-feed escape, because the master is the failing party.
    B,
    /// The master seat's ordinary report chain.
    A,
}

/// **ORDERED**: carrier B is tried first, carrier A otherwise.
///
/// `master_code` is the master seat's `queue_remedy_code` — **the remedy code, never the raw
/// `queue_blocked_by` reason**. The two are different vocabularies: of the 11 person codes only three share a
/// prefix with the raw reason that produces them, so a filter written against raw reasons selects three of
/// eleven classes on its best day. `queue_remedy_code` is also the one classification field present on every
/// consumer path (status JSON, queue list, routed summary).
///
/// `hang_observed` is the pre-existing hang test (two spaced observations plus a screen read), not a new
/// mechanism. A **calm-code hang** satisfies carrier A's literal predicate *and* carrier B's, which is why
/// this function is ordered rather than a pair of independent predicates.
pub(crate) fn select_carrier(master_code: Option<&str>, hang_observed: bool) -> Carrier {
    if eligible(master_code) || hang_observed {
        Carrier::B
    } else {
        Carrier::A
    }
}

// ---------------------------------------------------------------------------------------------------
// m0 / O-7 — THE DURABLE INCIDENT RECORD
// ---------------------------------------------------------------------------------------------------

/// K — CHOSEN, NOT MEASURED (O-6).
pub(crate) const CAP_PER_24H: u32 = 6;
/// 24 h in seconds — CHOSEN, NOT MEASURED (O-6). A reminder interval, not a measured flood limit.
pub(crate) const CAP_WINDOW_SECS: f64 = 86_400.0;
/// One re-approach per cooldown — CHOSEN, NOT MEASURED (O-6).
pub(crate) const REAPPROACH_COOLDOWN_SECS: f64 = 3_600.0;

/// The outcome written when the owner did not answer within the bounded wait.
pub(crate) const OUTCOME_UNANSWERED: &str = "unanswered";

/// What the selector may do next with an incident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reapproach {
    /// A fresh observation may carry a NEW item to the owner; this is attempt `attempt`.
    Allow { attempt: u32 },
    /// Inside the cooldown: not yet, and nothing is lost.
    CooldownHold,
    /// The cap is reached. The incident DOWNGRADES to carrier A in the state `unanswered × K`.
    DowngradeToCarrierA { state: String },
}

/// One escalation incident, keyed by the stable incident id.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Incident {
    /// The stable id — derived from the episode key, the same across every approach and both carriers.
    pub incident_id: String,
    /// How many times the owner has been approached about THIS incident.
    pub attempts: u32,
    /// `None` while open; `Some(OUTCOME_UNANSWERED)` once a bounded wait expired unanswered.
    pub outcome: Option<String>,
    /// Which carrier last held it.
    pub carrier: &'static str,
    /// Epochs of the approaches inside the cap window, oldest first.
    pub approaches: Vec<f64>,
}

impl Incident {
    pub(crate) fn new(incident_id: &str) -> Self {
        Self {
            incident_id: incident_id.to_string(),
            attempts: 0,
            outcome: None,
            carrier: "B",
            approaches: Vec::new(),
        }
    }

    /// Record that a bounded wait expired without an answer. Terminal for that ITEM, not for the incident.
    pub(crate) fn record_unanswered(&mut self, at: f64) {
        self.attempts = self.attempts.saturating_add(1);
        self.outcome = Some(OUTCOME_UNANSWERED.to_string());
        self.approaches.push(at);
    }

    /// Whether a FRESH OBSERVATION may carry a new item now. Never called by a timer: the caller is the
    /// ordered selector, firing on an observation of the same incident id.
    pub(crate) fn may_reapproach(&self, now: f64) -> Reapproach {
        let recent = self
            .approaches
            .iter()
            .filter(|t| now - **t < CAP_WINDOW_SECS)
            .count() as u32;
        if recent >= CAP_PER_24H {
            return Reapproach::DowngradeToCarrierA {
                state: format!("{OUTCOME_UNANSWERED} × {CAP_PER_24H}"),
            };
        }
        if let Some(last) = self.approaches.last() {
            if now - *last < REAPPROACH_COOLDOWN_SECS {
                return Reapproach::CooldownHold;
            }
        }
        Reapproach::Allow { attempt: self.attempts.saturating_add(1) }
    }

    /// The owner-facing line. The attempt count is IN the text, so the owner reads "third time" rather than an
    /// identical card.
    pub(crate) fn owner_text(&self, seats: &str) -> String {
        let n = self.attempts.saturating_add(1);
        format!(
            "[S5] 사람만 풀 수 있는 큐 막힘 — {seats} · {n}번째 알림(앞선 {prev}번은 응답 없이 만료됐습니다; \
만료된 알림은 승인도 거부도 아니며, 만료 뒤의 회신은 아무것도 승인하지 않습니다). \
알림은 쿨다운마다 한 번, 24시간에 최대 {k}번까지이고, 그 상한에 닿으면 이 사건은 사라지지 않고 \
`{unanswered} × {k}` 상태로 master 보고 경로로 내려갑니다.",
            seats = seats,
            n = n,
            prev = self.attempts,
            k = CAP_PER_24H,
            unanswered = OUTCOME_UNANSWERED,
        )
    }

    fn to_line(&self) -> String {
        let approaches: Vec<String> = self.approaches.iter().map(|t| format!("{t}")).collect();
        format!(
            "{{\"incident_id\":{},\"attempts\":{},\"outcome\":{},\"carrier\":{},\"approaches\":[{}]}}",
            json_str(&self.incident_id),
            self.attempts,
            self.outcome.as_deref().map_or("null".to_string(), json_str),
            json_str(self.carrier),
            approaches.join(",")
        )
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// What a read of the store found, including what it could not parse.
#[derive(Debug, Default)]
pub(crate) struct StoreRead {
    /// The latest record per incident id, in first-seen order.
    pub incidents: Vec<Incident>,
    /// Lines that could not be parsed — COUNTED AND PRESERVED, never dropped and never rewritten.
    pub unparsable: Vec<String>,
}

/// An append-only, per-line-tolerant record of escalation incidents.
///
/// **It never rewrites and never truncates.** Three rules, each of them a defect this product has already
/// paid for:
/// 1. append only — a whole-file rewrite is how a restore loses everything it was protecting;
/// 2. per-line tolerance — an unparsable line is preserved and skipped, never a reason to drop the rest;
/// 3. write errors are RETURNED, never discarded — a store whose failures are swallowed teaches nobody
///    anything about its own durability.
pub(crate) struct IncidentStore {
    path: PathBuf,
}

impl IncidentStore {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Append one record. **The error is returned**, so a caller can report it; it must never be used to
    /// refuse, block or gate anything.
    pub(crate) fn append(&self, incident: &Incident) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = incident.to_line();
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
        f.sync_all()?;
        Ok(())
    }

    /// Read the store. Byte-tolerant: the file is read as BYTES and split on `\n`, so a single invalid UTF-8
    /// line cannot make the whole read fail — which is exactly the all-or-nothing failure the feed restore
    /// has. A missing file is an empty store, not an error.
    pub(crate) fn read(&self) -> std::io::Result<StoreRead> {
        let raw = match std::fs::read(&self.path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(StoreRead::default()),
            Err(e) => return Err(e),
        };
        let mut out = StoreRead::default();
        for chunk in raw.split(|b| *b == b'\n') {
            if chunk.is_empty() {
                continue;
            }
            let Ok(text) = std::str::from_utf8(chunk) else {
                out.unparsable.push(String::from_utf8_lossy(chunk).into_owned());
                continue;
            };
            match parse_line(text) {
                Some(inc) => {
                    if let Some(slot) = out.incidents.iter_mut().find(|i| i.incident_id == inc.incident_id) {
                        *slot = inc; // the latest line for an id wins; earlier lines stay in the file
                    } else {
                        out.incidents.push(inc);
                    }
                }
                None => out.unparsable.push(text.to_string()),
            }
        }
        Ok(out)
    }
}

/// Minimal, dependency-free parse of one record line. Returns `None` for anything it cannot read, and the
/// caller preserves that line.
fn parse_line(text: &str) -> Option<Incident> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let id = v.get("incident_id")?.as_str()?.to_string();
    let attempts = v.get("attempts")?.as_u64()? as u32;
    let outcome = match v.get("outcome") {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        _ => None,
    };
    let carrier = match v.get("carrier").and_then(|c| c.as_str()) {
        Some("A") => "A",
        _ => "B",
    };
    let approaches = v
        .get("approaches")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|t| t.as_f64()).collect())
        .unwrap_or_default();
    Some(Incident { incident_id: id, attempts, outcome, carrier, approaches })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cys-s5-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // ---------------- O-1: the one compiled list, and the control that proves the test bites -------------

    #[test]
    fn s5_disjointness_every_code_classified_exactly_once_and_the_halves_are_disjoint() {
        let codes = crate::governance::QUEUE_REMEDY_CODES;
        assert_eq!(QUEUE_CODE_CLASS.len(), codes.len(), "the compiled list must cover every remedy code");
        // exactly once
        for (code, _) in QUEUE_CODE_CLASS {
            let n = QUEUE_CODE_CLASS.iter().filter(|(c, _)| *c == code).count();
            assert_eq!(n, 1, "{code} is classified {n} times, not once");
            assert!(codes.contains(&code), "{code} is not a real remedy code");
        }
        for code in codes {
            assert!(
                QUEUE_CODE_CLASS.iter().any(|(c, _)| *c == code),
                "{code} exists in QUEUE_REMEDY_CODES and is missing from the compiled list"
            );
        }
        // disjoint halves
        let person: Vec<&str> = QUEUE_CODE_CLASS.iter().filter(|(_, k)| *k == CodeClass::Person).map(|(c, _)| *c).collect();
        let calm: Vec<&str> = QUEUE_CODE_CLASS.iter().filter(|(_, k)| *k == CodeClass::Calm).map(|(c, _)| *c).collect();
        assert_eq!(person.len() + calm.len(), 14, "person {} + calm {}", person.len(), calm.len());
        for p in &person {
            assert!(!calm.contains(p), "{p} is in both halves");
        }
        assert_eq!(calm, vec!["paused", "approval", "wait"], "the calm half is exactly the three self-clearing codes");
        assert_eq!(person.len(), 11, "the person half is the 11 the app's own reader treats as person-held");
        // the S6 column, once filled, must be a SUBSET of the person half and must shrink eligibility by
        // exactly its own size.
        for c in S6_CLEARABLE {
            assert!(person.contains(&c), "S6 column entry {c} is not a person code");
        }
        let eligible_now = person.iter().filter(|c| eligible(Some(c))).count();
        assert_eq!(
            eligible_now,
            person.len() - S6_CLEARABLE.len(),
            "the S6 column must shrink eligibility by exactly its own size"
        );
    }

    /// THE CONTROL CASE, and it matters more than the pass: add a member to ONE side only and the invariant
    /// must go red. The real list is not touched — the control is a local copy, so the control proves the
    /// RULE bites rather than proving the list is currently fine.
    #[test]
    fn s5_disjointness_control_a_one_sided_addition_goes_red() {
        let mut one_sided: Vec<(&str, CodeClass)> = QUEUE_CODE_CLASS.to_vec();
        one_sided.push(("human_draft", CodeClass::Calm)); // a member added to one side only
        let person: Vec<&str> = one_sided.iter().filter(|(_, k)| *k == CodeClass::Person).map(|(c, _)| *c).collect();
        let calm: Vec<&str> = one_sided.iter().filter(|(_, k)| *k == CodeClass::Calm).map(|(c, _)| *c).collect();
        let overlap: Vec<&&str> = person.iter().filter(|p| calm.contains(p)).collect();
        assert!(!overlap.is_empty(), "the control must produce an overlap, otherwise the rule cannot bite");
        let dupes = one_sided.iter().filter(|(c, _)| *c == "human_draft").count();
        assert_eq!(dupes, 2, "the control must break exactly-once as well");
        // and the real list must still be clean
        assert!(QUEUE_CODE_CLASS.iter().filter(|(c, _)| *c == "human_draft").count() == 1);
    }

    // ---------------- O-5 + m3: the ordered selector ----------------------------------------------------

    #[test]
    fn s5_selector_is_ordered_b_first_and_a_calm_code_hang_is_distinguished_from_ineligible() {
        // a person code on the master seat ⇒ B
        assert_eq!(select_carrier(Some("answer_modal"), false), Carrier::B);
        // ★the case m3 named: a CALM code plus an observed hang satisfies BOTH literal predicates.
        // The ordered selector must give B, not A.
        assert_eq!(
            select_carrier(Some("wait"), true),
            Carrier::B,
            "a calm-code hang must select B: A's predicate (!eligible) is also true, and order decides"
        );
        // the ineligible case, which the calm-code hang must NOT be confused with
        assert_eq!(select_carrier(Some("wait"), false), Carrier::A);
        assert_eq!(select_carrier(Some("paused"), false), Carrier::A);
        // not blocked at all
        assert_eq!(select_carrier(None, false), Carrier::A);
        // fail-closed: a code outside the 14 is a person's problem
        assert_eq!(select_carrier(Some("a_code_from_a_newer_daemon"), false), Carrier::B);
    }

    // ---------------- m0 / O-7: expiry, durability, the cap -------------------------------------------

    #[test]
    fn s5_carrier_b_expiry_records_unanswered_durably_and_survives_a_restart() {
        let dir = tmpdir("restart");
        let path = dir.join("s5-incidents.jsonl");
        let id = "s5:surfaces=37,42:first=1791790000";

        // first process: the bounded wait expires unanswered
        {
            let store = IncidentStore::new(&path);
            let mut inc = Incident::new(id);
            inc.record_unanswered(1_791_790_123.0);
            store.append(&inc).expect("append must succeed and its error must be returned, not discarded");
        }
        // "restart": a brand-new store object over the same path, nothing carried in memory
        let store2 = IncidentStore::new(&path);
        let read = store2.read().expect("read");
        assert_eq!(read.incidents.len(), 1, "exactly one incident id");
        let got = &read.incidents[0];
        assert_eq!(got.incident_id, id, "the STABLE incident id survives");
        assert_eq!(got.attempts, 1, "the attempt count survives the restart");
        assert_eq!(got.outcome.as_deref(), Some(OUTCOME_UNANSWERED), "the outcome survives the restart");
        assert!(read.unparsable.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn s5_store_is_append_only_and_tolerates_one_bad_line_without_losing_the_others() {
        let dir = tmpdir("tolerance");
        let path = dir.join("s5-incidents.jsonl");
        let store = IncidentStore::new(&path);
        let mut a = Incident::new("incident-a");
        a.record_unanswered(1_000.0);
        store.append(&a).unwrap();

        // a corrupt line lands in the middle: invalid UTF-8 AND unparsable JSON
        {
            let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
            f.write_all(&[0x7b, 0xff, 0xfe, 0x22, 0x0a]).unwrap(); // `{` + invalid bytes + `"` + newline
            f.write_all(b"{not json at all}\n").unwrap();
        }
        let mut b = Incident::new("incident-b");
        b.record_unanswered(2_000.0);
        store.append(&b).unwrap();

        let before = std::fs::read(&path).unwrap();
        let read = store.read().expect("a bad line must not fail the read");
        assert_eq!(read.incidents.len(), 2, "both good records survive a bad line between them");
        assert_eq!(read.unparsable.len(), 2, "the bad lines are COUNTED, not dropped silently");
        let after = std::fs::read(&path).unwrap();
        assert_eq!(before, after, "reading must never rewrite the file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn s5_carrier_b_cap_control_one_the_k_plus_first_approach_does_not_reach_the_owner() {
        let mut inc = Incident::new("capped");
        let t0 = 1_791_700_000.0;
        // K approaches, each one cooldown apart, all inside the 24 h window
        for i in 0..CAP_PER_24H {
            let at = t0 + f64::from(i) * (REAPPROACH_COOLDOWN_SECS + 1.0);
            assert!(
                matches!(inc.may_reapproach(at), Reapproach::Allow { .. }),
                "approach {i} must be allowed"
            );
            inc.record_unanswered(at);
        }
        assert_eq!(inc.attempts, CAP_PER_24H);
        // the (K+1)-th, well past the cooldown and still inside 24 h
        let at = t0 + f64::from(CAP_PER_24H) * (REAPPROACH_COOLDOWN_SECS + 1.0);
        match inc.may_reapproach(at) {
            Reapproach::DowngradeToCarrierA { state } => {
                assert_eq!(state, format!("{OUTCOME_UNANSWERED} × {CAP_PER_24H}"));
            }
            other => panic!("the (K+1)-th approach must NOT reach the owner, got {other:?}"),
        }
    }

    #[test]
    fn s5_carrier_b_cap_control_two_at_the_cap_the_incident_reaches_carrier_a_as_unanswered_times_k() {
        let mut inc = Incident::new("downgraded");
        let t0 = 1_791_700_000.0;
        for i in 0..CAP_PER_24H {
            inc.record_unanswered(t0 + f64::from(i) * (REAPPROACH_COOLDOWN_SECS + 1.0));
        }
        let at = t0 + f64::from(CAP_PER_24H) * (REAPPROACH_COOLDOWN_SECS + 1.0);
        let Reapproach::DowngradeToCarrierA { state } = inc.may_reapproach(at) else {
            panic!("at the cap the incident must DOWNGRADE, not disappear");
        };
        // it DOWNGRADES: carrier A takes it, in an explicit state that names the silence and its count
        assert_eq!(state, "unanswered × 6");
        assert!(state.contains(OUTCOME_UNANSWERED) && state.contains(&CAP_PER_24H.to_string()));
        // and the incident is still in the record, with its attempts intact — it did not vanish
        assert_eq!(inc.attempts, CAP_PER_24H);
        assert_eq!(inc.outcome.as_deref(), Some(OUTCOME_UNANSWERED));
    }

    #[test]
    fn s5_reapproach_is_held_by_the_cooldown_and_is_never_fired_by_a_timer() {
        let mut inc = Incident::new("cooldown");
        let t0 = 1_791_700_000.0;
        inc.record_unanswered(t0);
        assert_eq!(inc.may_reapproach(t0 + 1.0), Reapproach::CooldownHold, "inside the cooldown");
        assert!(matches!(inc.may_reapproach(t0 + REAPPROACH_COOLDOWN_SECS + 1.0), Reapproach::Allow { attempt: 2 }));
        // ★the shape of the API is the proof that a timer cannot fire it: may_reapproach only ANSWERS a
        // caller that already has a fresh observation; nothing in this module schedules, sleeps or ticks.
        // ★The needles are SPLIT with concat! — a source pin whose own needles appear in its own source
        // matches itself and can only ever fail. The house pins in this codebase split theirs for the same
        // reason, and I rebuilt the defect here before catching it.
        let src = include_str!("s5_escalation.rs");
        for forbidden in [
            concat!("tokio", "::time"),
            concat!("thread", "::sleep"),
            concat!("interval", "("),
            concat!("Instant", "::now"),
        ] {
            assert!(!src.contains(forbidden), "this module must not contain a timer: found {forbidden}");
        }
    }

    #[test]
    fn s5_owner_text_carries_the_attempt_count_and_the_non_authorising_expiry() {
        let mut inc = Incident::new("texty");
        inc.record_unanswered(1_000.0);
        inc.record_unanswered(2_000.0);
        let text = inc.owner_text("surface:37");
        assert!(text.contains("3번째"), "the owner must read the attempt number: {text}");
        assert!(text.contains("승인도 거부도 아니며"), "expiry is neither an approval nor a denial: {text}");
        assert!(text.contains("아무것도 승인하지 않습니다"), "a post-expiry reply authorises nothing: {text}");
        assert!(text.contains("unanswered × 6"), "the downgrade state is in the owner's words: {text}");
    }

    // ---------------- source pins: the guarantees the PRODUCT already enforces --------------------------

    /// O-7's third requirement is a test over an EXISTING guard, not a change: a reply to an item that is not
    /// `pending` is refused. Pinned by source because the guarantee is structural — if this branch is ever
    /// removed, a post-expiry reply could authorise.
    #[test]
    fn s5_post_expiry_reply_authorises_nothing_source_pin() {
        let handlers = include_str!("handlers.rs");
        let reply = handlers
            .find("\"feed.reply\" =>")
            .expect("the feed.reply arm must exist");
        let window = &handlers[reply..reply + 3_000];
        assert!(
            window.contains("item.status != \"pending\""),
            "feed.reply must refuse an item that is not pending"
        );
        assert!(
            window.contains("item already resolved"),
            "the refusal must say why"
        );
        let main_rs = include_str!("main.rs");
        assert!(
            main_rs.contains("item.status = \"timeout\".into();"),
            "the daemon owns the pending -> timeout transition"
        );
        assert!(
            main_rs.contains(".filter(|i| i.status == \"pending\")"),
            "the timeout path must only rewrite an item that is still pending"
        );
    }

    /// m5 — the visible-draft exception. A zero-count VISIBLE human draft must be `human_draft`, and only the
    /// LATER zero-count arm may yield `wait`. The risk is a future reorder of two adjacent arms, which no
    /// behavioural test over one input can see, so the ORDER is pinned by source.
    #[test]
    fn s5_zero_count_visible_draft_is_never_suppressed_source_pin() {
        let gov = include_str!("governance.rs");
        let visible = gov
            .find("d.pending_input_bytes == 0 && d.draft_visible == Some(true)")
            .expect("the zero-count VISIBLE draft arm must exist");
        let bare = gov
            .find("} else if input_line && d.pending_input_bytes == 0 {")
            .expect("the bare zero-count arm must exist");
        assert!(
            visible < bare,
            "the visible-draft arm must come FIRST, or a person's unsent words are classified `wait` and \
             suppressed: visible at {visible}, bare zero-count at {bare}"
        );
        let arm = &gov[visible..bare];
        assert!(arm.contains("human_draft"), "the visible arm must yield human_draft");
        assert!(eligible(Some("human_draft")), "human_draft must be eligible for escalation");
        assert!(!eligible(Some("wait")), "wait must not be");
    }
}
