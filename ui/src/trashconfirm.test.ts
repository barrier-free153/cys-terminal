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
  type TrashEntry,
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
  ]) {
    const r = trashRefusalText(code);
    expect(r.text.length).toBeGreaterThan(0);
    expect(["preview_again", "none"]).toContain(r.action);
    expect(r.text).not.toContain("계속");
    expect(r.text).not.toContain("무시");
  }
  expect(trashRefusalText("uncertifiable-descendant-device").text).toContain("한 바이트도 지우지 않았습니다");
  expect(trashRefusalText("removal_error").text).toContain("남은 것이 있을 수 있습니다");
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
