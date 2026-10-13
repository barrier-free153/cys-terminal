// S8-UI 확인 로직 회귀 잠금 — 타이핑 확인이 항상 필요하고, 숫자를 꾸며내지 않고,
// 거부가 되돌릴 수 없는 쪽으로 가는 선택지를 주지 않는다.
import { expect, test } from "bun:test";
import {
  TRASH_UNAVAILABLE,
  trashAgeLines,
  trashBulkPhrase,
  trashChips,
  trashMismatchHint,
  trashModalLines,
  trashPhraseMatches,
  trashRefusalText,
  trashRequiredPhrase,
  trashSelectable,
  trashSizeText,
  trashUnavailableText,
  TRASH_ORDER_UNKNOWN,
  trashOutcomeText,
  trashOrderProgressText,
  trashOrderSeverity,
  type TrashEntry,
  type TrashEntryOutcome,
  type TrashOrder,
  TRASH_PREVIEW_EXPIRED_CODE,
  TRASH_DELETE_UNCERTAIN_CODE,
  TRASH_DELETE_REFUSED_TITLE,
  TRASH_DELETE_UNCERTAIN_TITLE,
  trashTransportReceipt,
} from "./trashconfirm";

const E = (o: Partial<TrashEntry> = {}): TrashEntry => ({
  name: "dept-4-20261008T010946Z",
  interrupted_deleting: false,
  deletable: true,
  root_mtime_age_secs: 4 * 86400,
  stamp_epoch: 1_760_000_000,
  old_by_stamp: false,
  children_depth1: ["account", "pack-dept-dept-4", "state"],
  children_total: 3,
  sole_copy_advisory: true,
  producer_chip: "",
  size: { state: "pending" },
  ...o,
});

test("확인 문구: 하나면 이름, 여럿이면 삭제 N — 클릭만으로 넘어가는 길은 없다", () => {
  expect(trashRequiredPhrase(["a-1"])).toBe("a-1");
  expect(trashRequiredPhrase(["a-1", "b-2"])).toBe("삭제 2");
  expect(trashBulkPhrase(3)).toBe("삭제 3");
  // 빈 선택은 어떤 입력으로도 통과하지 못한다.
  expect(trashPhraseMatches("", [])).toBe(false);
  expect(trashPhraseMatches("삭제 0", [])).toBe(false);
});

test("유일본 표지는 확인 강도를 낮추지 않는다(같은 문구를 요구한다)", () => {
  const sole = E({ sole_copy_advisory: true });
  const not = E({ sole_copy_advisory: false });
  expect(trashRequiredPhrase([sole.name])).toBe(trashRequiredPhrase([not.name]));
});

test("정규화는 resetconfirm 과 같다(따옴표·전각공백·NFD)", () => {
  expect(trashPhraseMatches('"삭제 2"', ["a", "b"])).toBe(true);
  expect(trashPhraseMatches("삭제　2", ["a", "b"])).toBe(true);
  expect(trashPhraseMatches("삭제 2 ", ["a", "b"])).toBe(true);
  expect(trashPhraseMatches("삭제2", ["a", "b"])).toBe(false);
  expect(trashMismatchHint("삭제2", ["a", "b"])).toContain("삭제 2");
  expect(trashMismatchHint("", ["a", "b"])).toBe("");
  expect(trashMismatchHint("삭제 2", ["a", "b"])).toBe("");
});

test("크기는 측정 전에 꾸며내지 않는다", () => {
  expect(trashSizeText({ state: "pending" })).toContain("미측정");
  expect(trashSizeText({ state: "measured", value: { bytes: 2_430_043_211, nodes: 27600 } })).toContain("2.4 GB");
});

test("날짜는 두 칸이다(유예 시계 = 이름의 스탬프 · mtime 은 활동)", () => {
  const now = 1_760_000_000 + 20 * 86400;
  const lines = trashAgeLines(E(), now);
  expect(lines[0]).toContain("휴지통에 들어간 지 20일");
  expect(lines[1]).toContain("내용이 마지막으로 바뀐 지 4일");
  expect(trashAgeLines(E({ stamp_epoch: null }), now)[0]).toContain("알 수 없음");
});

test("표지 순서가 고정이고, 멈춘 항목은 고를 수 없다", () => {
  const chips = trashChips(E({ old_by_stamp: true, interrupted_deleting: true, producer_chip: "producer is running" }));
  expect(chips).toEqual([
    "유일본 / sole copy",
    "오래됨 — 그대로 보관 중(명령해야 지워집니다)",
    "지우던 중 멈춤 — 다시 명령해야 합니다",
    "producer is running",
  ]);
  expect(trashSelectable(E({ deletable: false }))).toBe(false);
  expect(trashSelectable(E({ deletable: true }))).toBe(true);
});

test("거부 문구는 되돌릴 수 없는 쪽으로 가는 선택지를 주지 않는다", () => {
  for (const code of [
    "confirm_expired",
    "recent_may_be_in_use",
    "names_not_in_preview",
    "entry_changed_since_preview",
    "uncertifiable-descendant-device",
    "uncertifiable-unreadable",
    "uncertifiable-too-many-nodes",
    "producer_active",
    "refusal_record_unwritable",
    "busy",
    // ★S8 재착륙 §1 — 비동기 미리보기 잡의 코드 다섯(기본값으로 떨어지면 사유가 코드 문자열로 샌다).
    "preview_busy",
    "job_unknown",
    TRASH_PREVIEW_EXPIRED_CODE,
    "cannot_certify_in_time",
    "preview_worker_unavailable",
  ]) {
    const r = trashRefusalText(code);
    expect(r.text.length).toBeGreaterThan(0);
    expect(["preview_again", "none"]).toContain(r.action);
    expect(r.text).not.toContain("계속");
    expect(r.text).not.toContain("무시");
  }
  expect(trashRefusalText("uncertifiable-descendant-device").text).toContain("한 바이트도 지우지 않았습니다");
  expect(trashRefusalText("removal_error").text).toContain("남은 것이 있을 수 있습니다");
  // ★다섯 코드가 **기본값이 아니다**: 기본값은 코드 문자열을 괄호에 넣어 보여주므로 그것으로 가려낸다.
  for (const code of ["preview_busy", "job_unknown", TRASH_PREVIEW_EXPIRED_CODE, "cannot_certify_in_time",
                      "preview_worker_unavailable"]) {
    expect(trashRefusalText(code).text).not.toContain(code);
    expect(trashRefusalText(code).action).toBe("preview_again");
  }
  // 세션 상한 초과는 **한 번에 더 적게 고르라**고 말한다(같은 선택을 다시 눌러도 같은 결과다).
  expect(trashRefusalText("cannot_certify_in_time").text).toContain("더 적게");
  expect(trashRefusalText("preview_busy").text).toContain("다른 미리보기");
});

test("데몬이 낡으면 빈 휴지통이 아니라 오류를 보여준다", () => {
  expect(trashUnavailableText("unknown_method: trash.list 는 없는 동사다")).toBe(TRASH_UNAVAILABLE);
  expect(trashUnavailableText("connection refused")).toContain("connection refused");
});

test("화면 문구는 누가 허용되는지 말하지 않는다(데몬이 증명하지 않는 것을 주장하지 않는다)", () => {
  const all = [
    ...trashModalLines([E()], 1_760_000_000 + 86400),
    ...trashChips(E({ old_by_stamp: true, interrupted_deleting: true })),
    trashSizeText({ state: "pending" }),
    ...["confirm_expired", "owner_gui_required", "base_daemon_only"].map((c) => trashRefusalText(c).text),
  ].join("\n");
  const forbidden = ["오너만", "owner-only", "주인님만", "계정에 묶", "소유자만", "owner only"];
  for (const f of forbidden) {
    expect(all).not.toContain(f);
  }
});

test("모달은 지울 것과 되돌릴 수 없음을 적고, 공장초기화 항목에는 복구 원본 고지를 붙인다", () => {
  const plain = trashModalLines([E()], 1_760_000_000).join("\n");
  expect(plain).toContain("이 삭제는 되돌릴 수 없습니다.");
  expect(plain).not.toContain("되돌릴 때 쓰는 원본");
  const fr = trashModalLines([E({ name: "factory-reset-20260912T090920Z" })], 1_760_000_000).join("\n");
  expect(fr).toContain("되돌릴 때 쓰는 원본");
  expect(plain).toContain("안에 있는 것(첫 단계 3/3): account, pack-dept-dept-4, state");
});

// ─── 결과 문구 표(outcome 하나당 한 줄) ────────────────────────────────────────────────────────
const O = (o: Partial<TrashEntryOutcome> = {}): TrashEntryOutcome => ({
  name: "dept-4-20261008T010946Z",
  outcome: "deleted_verified",
  reason_code: null,
  boundary: null,
  bytes_before: 1024,
  duration_ms: 12,
  remaining: null,
  errors: [],
  ...o,
});

test("결과 표: 지운 것과 이미 없던 것만 ok — 나머지는 warn/fail 이고 모르는 outcome 은 fail", () => {
  expect(trashOutcomeText(O()).severity).toBe("ok");
  expect(trashOutcomeText(O()).text).toContain("확인했습니다");
  expect(trashOutcomeText(O({ outcome: "already_gone" })).severity).toBe("ok");
  // 거부는 사유 문구를 refusal 표에서 가져온다(두 곳에 같은 문장을 쓰지 않는다).
  const ref = trashOutcomeText(O({ outcome: "refused", reason_code: "confirm_expired" }));
  expect(ref.severity).toBe("warn");
  expect(ref.text).toContain(trashRefusalText("confirm_expired").text);
  // ★모르는 outcome 을 성공으로 접지 않는다.
  const unknown = trashOutcomeText(O({ outcome: "something_new_from_a_newer_daemon" }));
  expect(unknown.severity).toBe("fail");
  expect(unknown.text).toContain("something_new_from_a_newer_daemon");
});

test("결과 표: 봉인 단계 중단은 '한 바이트도 안 지웠다'고 말하고 스스로 이어가지 않는다고 말한다", () => {
  const t = trashOutcomeText(O({ outcome: "sealed_not_removed", reason_code: "entry_changed_since_preview" }));
  expect(t.severity).toBe("fail");
  expect(t.text).toContain("한 바이트도 지우지 않았고");
  expect(t.text).toContain(".deleting-");
  expect(t.text).toContain("새로 미리보기");
});

test("결과 표: 일부만 지워진 것은 남은 수를 모를 때 '알 수 없음' 이라고 적는다(0 으로 꾸미지 않는다)", () => {
  const known = trashOutcomeText(O({ outcome: "partial", remaining: 7, errors: ["Permission denied"] }));
  expect(known.severity).toBe("fail");
  expect(known.text).toContain("남은 것 7개");
  expect(known.text).toContain("Permission denied");
  const unknown = trashOutcomeText(O({ outcome: "partial", remaining: null, errors: [] }));
  expect(unknown.text).toContain("남은 것 알 수 없음");
  expect(unknown.text).not.toContain("0개");
});

test("진행 문구: finished 가 없으면 '삭제 중' — 오래 걸리는 것은 멈춘 것이 아니라고 적는다", () => {
  const base: TrashOrder = { order_id: "or-1", started: 1, finished: null, running_long: false, per_entry: [] };
  expect(trashOrderProgressText(base, 3)).toBe("삭제 중 0/3");
  expect(trashOrderProgressText({ ...base, per_entry: [O()] }, 3)).toBe("삭제 중 1/3");
  expect(trashOrderProgressText({ ...base, running_long: true }, 3)).toContain("멈추지 않았습니다");
  expect(trashOrderProgressText({ ...base, finished: 2, per_entry: [O(), O()] }, 2)).toBe("끝났습니다 — 2/2 항목 처리");
});

test("명령 심각도 = 항목 중 가장 나쁜 것 · 결과를 못 읽었을 때는 성공도 실패도 주장하지 않는다", () => {
  const mk = (per: TrashEntryOutcome[]): TrashOrder => ({
    order_id: "or-1", started: 1, finished: 2, running_long: false, per_entry: per,
  });
  expect(trashOrderSeverity(mk([]))).toBe("ok");
  expect(trashOrderSeverity(mk([O(), O({ outcome: "already_gone" })]))).toBe("ok");
  expect(trashOrderSeverity(mk([O(), O({ outcome: "refused", reason_code: "busy" })]))).toBe("warn");
  expect(trashOrderSeverity(mk([O({ outcome: "refused" }), O({ outcome: "partial" })]))).toBe("fail");
  expect(TRASH_ORDER_UNKNOWN).toContain("DELETE-LEDGER.jsonl");
  for (const bad of ["지웠습니다", "실패했습니다"]) expect(TRASH_ORDER_UNKNOWN).not.toContain(bad);
});

// ───────────────────────────────────────────────────────────────────────────────────────────
// ★MJR-4 결박 — `test_delete_transport_loss_receipt_is_unknown_not_refused` 의 **문면 절반**.
//   델타 §2.3 의 6단정 중 ⑴제목 ⑵상태 ⑶본문 ⑷제목 금지어 ⑸본문 금지어 다섯이 여기 있고,
//   ⑹전송 1회(자동 재전송 0)는 소켓을 봐야 하므로 `src-tauri` 의 같은 이름 검체가 잰다.
//   ★판정이 순수 함수에 있으므로 이것은 **문자열 핀이 아니라 동작 검사**다.
// ───────────────────────────────────────────────────────────────────────────────────────────
test("test_delete_transport_loss_receipt_is_unknown_not_refused — 전송 불확실은 거부가 아니다", () => {
  const r = trashTransportReceipt(TRASH_DELETE_UNCERTAIN_CODE, "transport_uncertain: connection closed");
  // ⑴ 제목 ⑵ 상태 ⑶ 본문
  expect(r.title).toBe("휴지통 삭제 — 결과를 확인할 수 없습니다");
  expect(r.status).toBe(TRASH_ORDER_UNKNOWN);
  expect(r.body).toContain("명령 바이트가 전송되었으나");
  // ⑷⑸ ★엄격 금지선 — 제목에도 본문에도 그 말이 없어야 한다(있으면 거짓 주장이다)
  expect(r.title).not.toContain("지우지 않았습니다");
  expect(r.body).not.toContain("지우지 않았습니다");
  // 화해 프로토콜 — 맹목 재전송 차단
  expect(r.klass).toBe("uncertain");
  expect(r.resend).toBe("blocked");
  // 다시 누르지 말라는 말이 본문에 있다(원장 대조가 먼저다)
  expect(r.body).toContain("다시 삭제를 누르지 마십시오");
});

test("★양성 대조(Class 1) — BeforeSend·정형 거부는 '지우지 않았습니다'로 정확히 분기한다", () => {
  for (const code of ["confirm_expired", "entry_changed_since_preview", "producer_active", "connection refused"]) {
    const r = trashTransportReceipt(code, `${code}: x`);
    expect(r.title).toBe("휴지통 삭제 — 지우지 않았습니다");
    expect(r.klass).toBe("refused");
    expect(r.resend).toBe("allowed");
    expect(r.status).not.toBe(TRASH_ORDER_UNKNOWN);
  }
  // ★두 부류의 제목은 **다르다**(같아지면 이 검사 전체가 무의미해진다)
  expect(TRASH_DELETE_REFUSED_TITLE).not.toBe(TRASH_DELETE_UNCERTAIN_TITLE);
});
