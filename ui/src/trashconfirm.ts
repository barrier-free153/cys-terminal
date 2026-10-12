// S8-UI 휴지통 삭제 확인의 **순수 판정·문구 조립** — main.ts 는 배선만 한다.
// resetconfirm.ts 규약 계승(타이핑 확인·입력 자동교정 차단·불일치 사유 표시). 판정을 순수 함수로
// 고정해 결정론 회귀 시험으로 잠근다.
//
// ★이 파일이 지키는 세 가지 금지
//  ① **클릭만으로 넘어가는 단계 없음** — 항목이 하나든 여럿이든 **타이핑 확인이 항상** 필요하다.
//     유일본 표지는 **조언**이므로 확인의 강도를 낮추지 않는다(표지가 틀려도 관문은 그대로다).
//  ② **전체선택 없음** — 선택은 항목마다 손으로 한다('오래된 것 전부' 단추도 없다).
//  ③ **누가 허용되는지 말하지 않는다** — 화면 문구는 "이 도구가 무엇을 하는가"만 적는다
//     ("오너만 삭제할 수 있습니다" 같은 문장은 데몬이 증명하지 않는 것을 주장한다 · r2 §2.4).
import { normalizePhrase, resetSizeHuman } from "./resetconfirm";

/// 여러 항목을 지울 때 타이핑할 문구 — `삭제 N`.
export function trashBulkPhrase(count: number): string {
  return `삭제 ${count}`;
}

/// 이 선택에 대해 요구되는 확인 문구. 하나면 **그 항목의 이름**, 여럿이면 `삭제 N`.
export function trashRequiredPhrase(names: string[]): string {
  return names.length === 1 ? names[0] : trashBulkPhrase(names.length);
}

/// 입력이 요구 문구와 일치하는가(resetconfirm 과 **같은 정규화**를 쓴다 — 한쪽만 관용하면
/// 같은 입력이 한 화면에선 되고 다른 화면에선 안 되는 계약 분열이 생긴다).
export function trashPhraseMatches(input: string, names: string[]): boolean {
  if (names.length === 0) return false;
  return normalizePhrase(input) === normalizePhrase(trashRequiredPhrase(names));
}

/// 불일치 실시간 힌트(빈 입력·일치 = 빈 문자열).
export function trashMismatchHint(input: string, names: string[]): string {
  const want = trashRequiredPhrase(names);
  const v = normalizePhrase(input);
  if (!v || trashPhraseMatches(input, names)) return "";
  return `입력 "${v}" 는 "${want}" 와 다릅니다 — 띄어쓰기까지 그대로 입력하세요.`;
}

export type TrashSize =
  | { state: "pending" }
  | { state: "measured"; value: { bytes: number; nodes: number } };

export type TrashEntry = {
  name: string;
  interrupted_deleting: boolean;
  deletable: boolean;
  root_mtime_age_secs: number;
  stamp_epoch: number | null;
  old_by_stamp: boolean;
  children_depth1: string[];
  children_total: number;
  sole_copy_advisory: boolean;
  producer_chip: string;
  size: TrashSize;
};

/// 크기 칸의 사람용 문구 — **측정 전에는 숫자를 꾸며내지 않는다**(목록이 걷지 않기 때문에
/// pending 이 정상 상태다).
export function trashSizeText(size: TrashSize): string {
  if (size.state === "measured") {
    return `${resetSizeHuman(size.value.bytes)} · ${size.value.nodes.toLocaleString("ko-KR")}개 항목`;
  }
  return "크기 미측정 — 미리보기에서 측정합니다";
}

/// 휴지통에 들어간 시점(이름의 스탬프)과 내용이 마지막으로 바뀐 시점을 **두 칸으로** 보여준다.
/// 유예 시계는 이름의 스탬프이고, mtime 은 활동(최근) 판정에만 쓰인다 — 한 칸에 섞으면 둘 중
/// 하나가 반드시 거짓이 된다.
export function trashAgeLines(e: TrashEntry, nowEpoch: number): string[] {
  const out: string[] = [];
  if (e.stamp_epoch == null) {
    out.push("휴지통에 들어간 시점: 알 수 없음(이름에 스탬프가 없습니다)");
  } else {
    const d = Math.max(0, Math.floor((nowEpoch - e.stamp_epoch) / 86400));
    out.push(`휴지통에 들어간 지 ${d}일`);
  }
  const md = Math.max(0, Math.floor(e.root_mtime_age_secs / 86400));
  out.push(`내용이 마지막으로 바뀐 지 ${md}일`);
  return out;
}

/// 항목에 붙는 표지(칩) — 순서 고정(결정론 · 시험이 이 순서를 잠근다).
export function trashChips(e: TrashEntry): string[] {
  const chips: string[] = [];
  if (e.sole_copy_advisory) chips.push("유일본 / sole copy");
  if (e.old_by_stamp) chips.push("오래됨 — 그대로 보관 중(명령해야 지워집니다)");
  if (e.interrupted_deleting) chips.push("지우던 중 멈춤 — 다시 명령해야 합니다");
  if (e.producer_chip) chips.push(e.producer_chip);
  return chips;
}

/// 체크박스를 둘 수 있는가 — 데몬의 판정을 그대로 따른다(화면이 다시 판단하지 않는다).
export function trashSelectable(e: TrashEntry): boolean {
  return e.deletable;
}

/// 데몬 오류 코드 → 사람 문구와 **단 하나의 행동**. 되돌릴 수 없는 쪽으로 가는 선택지는 없다.
export function trashRefusalText(code: string): { text: string; action: "preview_again" | "none" } {
  switch (code) {
    case "confirm_expired":
      return { text: "확인 시간이 지났습니다 — 새로 미리보기를 하세요.", action: "preview_again" };
    case "recent_may_be_in_use":
      return { text: "이 항목이 방금 쓰였습니다(또는 판정할 수 없습니다) — 지우지 않았습니다.", action: "preview_again" };
    case "names_not_in_preview":
      return { text: "미리보기에 없던 항목입니다 — 새로 미리보기를 하세요.", action: "preview_again" };
    case "entry_changed_since_preview":
      return { text: "미리보기 뒤 내용이 바뀌었습니다 — 지우지 않았습니다.", action: "preview_again" };
    case "uncertifiable-descendant-device":
      return {
        text: "이 항목 안에 다른 장치(마운트)가 있어 **전체를 확인할 수 없습니다** — 한 바이트도 지우지 않았습니다.",
        action: "none",
      };
    case "uncertifiable-unreadable":
      return { text: "이 항목 안에 읽을 수 없는 것이 있어 확인할 수 없습니다 — 지우지 않았습니다.", action: "none" };
    case "uncertifiable-too-many-nodes":
    case "uncertifiable-too-slow":
      return { text: "이 항목이 너무 커서 정해진 한도 안에 확인할 수 없습니다 — 이 판에서는 지울 수 없습니다.", action: "none" };
    case "producer_active":
      return { text: "이 항목을 만든 작업이 아직 돌고 있습니다(또는 판정할 수 없습니다) — 지우지 않았습니다.", action: "preview_again" };
    case "refusal_record_unwritable":
      return { text: "기록을 남길 수 없어 진행하지 않았습니다 — 아무 것도 건드리지 않았습니다.", action: "none" };
    case "seal_failed":
      return { text: "지우기 전 단계에서 멈췄습니다 — 아무 것도 지우지 않았습니다.", action: "preview_again" };
    case "removal_error":
      return { text: "지우는 중 오류가 났습니다 — 남은 것이 있을 수 있습니다(아래 결과를 확인하세요).", action: "none" };
    case "busy":
      return { text: "다른 삭제가 진행 중입니다 — 끝난 뒤 다시 명령하세요.", action: "none" };
    case "unsupported_platform_pending_validation":
      return { text: "이 빌드에서는 삭제가 꺼져 있습니다(목록과 표시는 그대로 동작합니다).", action: "none" };
    case "base_daemon_only":
      return { text: "휴지통은 기계에 하나입니다 — 기본 데몬에서만 다룹니다.", action: "none" };
    case "owner_gui_required":
      return { text: "이 창에서만 내리는 명령입니다.", action: "none" };
    default:
      return { text: `처리하지 못했습니다 (${code}).`, action: "none" };
  }
}

/// 데몬이 없거나 너무 낡아 `trash.*` 를 모를 때 — **빈 휴지통으로 보여주지 않는다**.
export const TRASH_UNAVAILABLE = "휴지통을 열 수 없습니다 — 데몬을 업데이트하세요.";
export function trashUnavailableText(err: unknown): string {
  const s = String(err ?? "");
  return /unknown_method|trash\./.test(s) ? TRASH_UNAVAILABLE : `휴지통을 열 수 없습니다 — ${s}`;
}

/// 모달 본문 — 지울 것, 되돌릴 수 없다는 사실, 공장초기화 격리분이 복구 원본이라는 사실.
export function trashModalLines(entries: TrashEntry[], nowEpoch: number): string[] {
  const lines: string[] = [];
  for (const e of entries) {
    lines.push(`• ${e.name} — ${trashSizeText(e.size)}`);
    for (const l of trashAgeLines(e, nowEpoch)) lines.push(`    ${l}`);
    const chips = trashChips(e);
    if (chips.length) lines.push(`    ${chips.join(" · ")}`);
    const shown = e.children_depth1.slice(0, 50);
    lines.push(
      `    안에 있는 것(첫 단계 ${shown.length}/${e.children_total}): ${shown.join(", ") || "(없음)"}`,
    );
  }
  lines.push("");
  lines.push("이 삭제는 되돌릴 수 없습니다.");
  if (entries.some((e) => e.name.startsWith("factory-reset-"))) {
    lines.push("factory-reset- 로 시작하는 항목은 이전 설치를 되돌릴 때 쓰는 원본입니다.");
  }
  return lines;
}
