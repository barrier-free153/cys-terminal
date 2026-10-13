// S8-UI 휴지통 탭 배선 핀(main.ts · index.html 을 **데이터로 읽는다** · officewiring.test.ts 관례:
// 런타임 코드 0줄 · 주석 제거 본문 기준). 설계 12.U 가 요구한 네 가지를 여기서 잠근다 —
// ①탭 배선 ②모달의 타이핑 확인 ③걷는 동안의 안내 문구 ④낡은 데몬의 오류 문구.
// 판정·문구 자체는 trashconfirm.test.ts 가 표로 잡는다 — 여기는 main.ts 가 그것을 어떻게 옮기는가.
import { describe, it, expect } from "bun:test";
import { readFileSync } from "node:fs";

const read = (rel: string) => readFileSync(new URL(rel, import.meta.url), "utf-8");
const stripComments = (s: string): string =>
  s
    .split("\n")
    .map((l) => (l.trimStart().startsWith("//") ? "" : l.replace(/\s\/\/.*$/, "")))
    .join("\n");
const code = stripComments(read("./main.ts"));
const html = read("../index.html");
function fnBody(name: string): string {
  const i = code.search(new RegExp(`(async )?function ${name}\\(`));
  expect({ 함수: name, 존재: i >= 0 }).toEqual({ 함수: name, 존재: true });
  const end = code.indexOf("\n}\n", i);
  return code.slice(i, end > i ? end + 3 : undefined);
}
const panel = html.slice(html.indexOf('<div id="cc-view-trash"'), html.indexOf('<div id="cc-view-office"'));

describe("탭과 패널의 마크업(index.html)", () => {
  it("#cc-tabs 안에 data-view=trash 단추가 있고, 그에 대응하는 패널이 숨김으로 있다", () => {
    const tabs = html.slice(html.indexOf('<div id="cc-tabs">'), html.indexOf('<div id="cc-body">'));
    expect(/<button class="cc-tab" data-view="trash">/.test(tabs)).toBe(true);
    expect(/<div id="cc-view-trash" hidden>/.test(html)).toBe(true);
    for (const id of ["cc-trash-root", "cc-trash-notice", "cc-trash-list", "cc-trash-status", "cc-trash-result"])
      expect({ 칸: id, 있음: panel.includes(`id="${id}"`) }).toEqual({ 칸: id, 있음: true });
  });
  it("★패널의 단추는 **삭제 하나**다 — '전부 선택'·'오래된 것 전부' 단추가 마크업에 없다(Q2)", () => {
    const buttons = panel.match(/<button/g) ?? [];
    expect(buttons.length).toBe(1);
    expect(/<button id="btn-cc-trash-delete" disabled>/.test(panel)).toBe(true);
    for (const bad of ["select-all", "전부 선택", "모두 선택", "모두 삭제", "전부 삭제", "오래된 것 전부"])
      expect({ 금지: bad, 있음: panel.includes(bad) }).toEqual({ 금지: bad, 있음: false });
  });
  it("★안내 문구는 자동 삭제를 약속하지 않고, 누가 허용되는지 말하지 않는다(r2 §8 문안 규칙)", () => {
    for (const bad of ["자동으로 삭제", "자동 삭제", "14일", "일 후에 삭제", "오너만", "주인만", "owner-only", "오너 전용"])
      expect({ 금지: bad, 있음: panel.includes(bad) }).toEqual({ 금지: bad, 있음: false });
    // 하는 일은 적는다: 보관 · 고름 · 확인 입력 · 되돌릴 수 없음.
    for (const must of ["그대로 보관", "확인 문구", "되돌릴 수 없"])
      expect({ 필수: must, 있음: panel.includes(must) }).toEqual({ 필수: must, 있음: true });
  });
});

describe("탭 전환과 주기 갱신의 배선(main.ts)", () => {
  it("CcTab 에 trash 가 있고 setCcTab 이 패널을 토글하고 진입 시 목록을 읽는다", () => {
    expect(code.includes('| "board" | "tasks" | "feed" | "alarms" | "trash" | "office";')).toBe(true);
    const b = fnBody("setCcTab");
    expect(b.includes('document.getElementById("cc-view-trash")!.hidden = view !== "trash";')).toBe(true);
    expect(b.includes('if (view === "trash") void refreshTrash();')).toBe(true);
  });
  it("★m3: 진입 훅만이 아니라 **주기 라우팅**(refreshControlCenter)에도 있다 — 탭에 머물면 멈추는 목록이 아니다", () => {
    expect(fnBody("refreshControlCenter").includes('if (ccTab === "trash") void refreshTrash();')).toBe(true);
  });
  it("삭제 단추는 흐름 함수 하나에만 연결된다", () => {
    expect(code.includes('document.getElementById("btn-cc-trash-delete")?.addEventListener("click", () => void trashDeleteFlow());')).toBe(true);
  });
  it("새 setInterval 0 — 결과 읽기는 setTimeout 연쇄다(탭 틱 수 핀 불변)", () => {
    for (const f of ["refreshTrash", "trashDeleteFlow", "trashPollOrder", "trashStopPoll", "renderTrash",
                     "trashConfirmModal", "trashStopPreviewPoll"])
      expect({ 함수: f, setInterval: fnBody(f).includes("setInterval(") }).toEqual({ 함수: f, setInterval: false });
    expect(fnBody("trashPollOrder").includes("setTimeout(")).toBe(true);
  });
});

describe("데몬이 부르는 명령만 쓰고, 이름은 그대로 돌려보낸다 (★S8 재착륙 §1 — 동기 미리보기 퇴역)", () => {
  it("invoke 집합은 trash_list·trash_order·trash_delete + **비동기** preview 둘이고, 동기 trash_preview 는 없다", () => {
    const block = code.slice(code.indexOf("function trashStopPoll("), code.indexOf("async function refreshBoard("));
    const calls = [...block.matchAll(/invoke\("([a-z_]+)"/g)].map((m) => m[1]);
    expect([...new Set(calls)].sort()).toEqual([
      "trash_delete",
      "trash_list",
      "trash_order",
      "trash_preview_poll",
      "trash_preview_start",
    ]);
    // ★퇴역의 증거는 **이 파일 전체**에서 동기 이름이 사라진 것이다(모듈 어디서도 부르지 않는다).
    expect(/invoke\("trash_preview"/.test(code)).toBe(false);
  });
  it("미리보기 기동은 **고른 이름만** 보내고, 명령은 미리보기가 준 confirm_id·digest 를 되보낸다", () => {
    const m = fnBody("trashConfirmModal");
    expect(m.includes('invoke("trash_preview_start", { names })')).toBe(true);
    expect(m.includes('invoke("trash_preview_poll", { jobId })')).toBe(true);
    const f = fnBody("trashDeleteFlow");
    expect(f.includes("confirmId: prev.confirm_id")).toBe(true);
    expect(f.includes("digest: prev.digest")).toBe(true);
    expect(f.includes("names: confirmNames")).toBe(true);
  });
  it("★폴링은 500ms 타이머 연쇄다 — 새 setInterval 0 이고, 모달을 닫으면 타이머가 취소된다", () => {
    const m = fnBody("trashConfirmModal");
    expect(m.includes("setInterval(")).toBe(false);
    expect(m.includes("setTimeout(() => void poll(jobId), TRASH_PREVIEW_POLL_MS)")).toBe(true);
    expect(code.includes("const TRASH_PREVIEW_POLL_MS = 500;")).toBe(true);
    // 닫기·취소의 유일한 출구(`done`)가 폴링을 끊는다 — 낡은 연쇄는 세대 번호로도 한 번 더 막힌다.
    expect(m.includes("trashStopPreviewPoll();")).toBe(true);
    expect(fnBody("trashStopPreviewPoll").includes("clearTimeout(trashPreviewTimer);")).toBe(true);
    expect((m.match(/if \(gen !== trashPreviewGen\) return;/g) ?? []).length).toBeGreaterThanOrEqual(4);
  });
  it("★대기는 **보인다**: 스피너와 진행 수치(노드·경과초)가 모달 안에 있고, 타이핑은 ready 에서만 열린다", () => {
    const m = fnBody("trashConfirmModal");
    expect(m.includes('<span class="dept-spinner"></span>')).toBe(true);
    expect(m.includes('<input class="modal-input" type="text" disabled />')).toBe(true);
    expect(m.includes("노드 ${pg.nodes.toLocaleString()} · ${pg.elapsed_secs}초")).toBe(true);
    // 입력을 **여는** 곳은 ready 하나다(대기·거부 쪽에서는 전부 다시 잠근다).
    expect((m.match(/inp\.disabled = false;/g) ?? []).length).toBe(1);
    const iReady = m.indexOf("const ready = (prev: TrashPreview) => {");
    const iOpen = m.indexOf("inp.disabled = false;");
    expect(iReady >= 0 && iOpen > iReady).toBe(true);
    expect(m.includes("TRASH_PREVIEW_WAIT_LINE")).toBe(true);
  });
  it("★상태 전이표의 네 끝(ready·failed·expired·job_unknown)이 전부 다뤄진다", () => {
    const m = fnBody("trashConfirmModal");
    expect(m.includes('if (st.status === "running")')).toBe(true);
    expect(m.includes('if (st.status === "ready" && st.result)')).toBe(true);
    expect(m.includes('if (st.status === "expired")')).toBe(true);
    expect(m.includes("refuse(TRASH_PREVIEW_EXPIRED_CODE")).toBe(true);
    // job_unknown·preview_busy 는 Tauri 가 "code: message" 로 던지므로 코드만 떼어 표에 넣는다.
    expect((m.match(/refuse\(String\(e\)\.split\(":"\)\[0\]/g) ?? []).length).toBe(2);
    expect(m.includes('refuse(st.refusal?.reason_code ?? "unknown", st.refusal?.detail ?? "")')).toBe(true);
  });
});

describe("확인 관문 — 클릭만으로 넘어가는 길이 없다", () => {
  const modal = fnBody("trashConfirmModal");
  it("타이핑 확인은 순수 모듈의 판정으로만 열린다(자동교정 차단 · 불일치 사유 표시)", () => {
    // ★이름은 **데몬이 확인한 집합**(`confirmNames`)이다 — 고른 것 전부가 아니다(거부는 빠진다).
    expect(modal.includes("trashPhraseMatches(inp.value, confirmNames)")).toBe(true);
    expect(modal.includes("yes.disabled = !trashPhraseMatches")).toBe(true);
    expect(modal.includes("trashMismatchHint(inp.value, confirmNames)")).toBe(true);
    expect(modal.includes("for (const [k, v] of PURGE_INPUT_GUARDS) inp.setAttribute(k, v);")).toBe(true);
    expect(modal.includes('<button class="modal-yes" disabled>')).toBe(true);
  });
  it("★기본 포커스는 취소다 — Enter 한 번이 삭제로 가는 길을 두지 않는다(설계 6.2 3항)", () => {
    expect(modal.includes("no.focus()")).toBe(true);
    expect(modal.includes("inp.focus()")).toBe(false);
  });
  it("confirm() 은 쓰지 않는다(이 WebView 에서 억제된다 · 설계 6.3)", () => {
    for (const f of ["trashConfirmModal", "trashDeleteFlow", "trashPollOrder", "renderTrash"])
      expect({ 함수: f, confirm: /[^a-zA-Z.]confirm\(/.test(fnBody(f)) }).toEqual({ 함수: f, confirm: false });
  });
  it("모달의 innerHTML 은 **값이 섞이지 않은 뼈대**뿐이고 내용은 textContent 로 들어간다(이름은 파일 이름이다)", () => {
    const iv = modal.indexOf("ov.innerHTML =");
    const ivEnd = modal.indexOf("(ov.querySelector(\"h3\")", iv);
    expect(iv >= 0 && ivEnd > iv).toBe(true);
    expect(modal.slice(iv, ivEnd).includes("${")).toBe(false);
    // ★뼈대를 한 번만 만들고(대기 상태), 내용은 **나중에** textContent 로 들어간다 — ready 에서
    //   값이 섞인 innerHTML 을 다시 쓰지 않는다(이름은 파일 이름이다).
    expect(modal.includes('label.textContent = lines.join("\\n");')).toBe(true);
    expect((modal.match(/innerHTML/g) ?? []).length).toBe(1);
    for (const f of ["renderTrash", "renderTrashResult", "renderTrashUnavailable"])
      expect({ 함수: f, innerHTML: fnBody(f).includes("innerHTML") }).toEqual({ 함수: f, innerHTML: false });
    expect(fnBody("renderTrash").includes("name.textContent = e.name;")).toBe(true);
  });
  it("모달이 참을 돌려준 **뒤에만** 명령이 나간다 · 취소는 아무 것도 지우지 않는다고 말한다", () => {
    const f = fnBody("trashDeleteFlow");
    const iModal = f.indexOf("await trashConfirmModal(");
    const iOrder = f.indexOf('invoke("trash_delete"');
    expect(iModal >= 0 && iOrder > iModal).toBe(true);
    expect(f.includes("if (!ok || !prev?.confirm_id || !prev?.digest) {")).toBe(true);
    expect(f.includes("취소했습니다 — 아무 것도 지우지 않았습니다.")).toBe(true);
  });
  it("★선택은 항목마다 손으로 한다 — 선택 집합에 넣는 곳은 체크박스 처리기 **하나**다(전체선택이 코드에도 없다)", () => {
    const block = code.slice(code.indexOf("function trashStopPoll("), code.indexOf("async function refreshBoard("));
    expect((block.match(/trashSelected\.add\(/g) ?? []).length).toBe(1);
    expect(fnBody("renderTrash").includes("if (cb.checked) trashSelected.add(e.name);")).toBe(true);
    // 고를 수 없는 항목에는 체크박스를 만들지 않는다(사유는 표지가 말한다).
    expect(fnBody("renderTrash").includes("if (trashSelectable(e)) {")).toBe(true);
  });
});

describe("걷는 동안 · 데몬이 없을 때 · 결과를 못 읽을 때 (12.U ③④)", () => {
  it("미리보기 전에 '확인하는 중' 안내를 띄운다(무피드백이면 재클릭을 부른다)", () => {
    const f = fnBody("trashDeleteFlow");
    const iStatus = f.indexOf("고른 항목을 확인하는 중입니다");
    const iModal = f.indexOf("await trashConfirmModal(names)");
    expect(iStatus >= 0 && iModal > iStatus).toBe(true);
    // ★모달 쪽에서도 대기가 먼저다: 뼈대를 붙이고 대기 줄을 적은 **뒤에** 잡을 띄운다
    //   (먼저 띄우면 ready 가 즉시 와도 화면이 한 프레임 비어 있다).
    const m = fnBody("trashConfirmModal");
    const iWait = m.indexOf("TRASH_PREVIEW_WAIT_LINE");
    const iStart = m.indexOf('invoke("trash_preview_start"');
    expect(iWait >= 0 && iStart > iWait).toBe(true);
  });
  it("★목록 실패는 **빈 휴지통으로 보이지 않는다** — 순수 모듈의 문구를 그대로 쓴다(R-7)", () => {
    const f = fnBody("refreshTrash");
    expect(f.includes("renderTrashUnavailable(trashUnavailableText(e));")).toBe(true);
    expect(fnBody("renderTrashUnavailable").includes("trashSyncDeleteButton(false)")).toBe(true);
  });
  it("결과를 끝까지 못 읽으면 성공도 실패도 주장하지 않고 원장을 가리킨다", () => {
    const f = fnBody("trashPollOrder");
    expect((f.match(/TRASH_ORDER_UNKNOWN/g) ?? []).length).toBeGreaterThanOrEqual(2);
    expect(f.includes("polls >= TRASH_POLL_MAX")).toBe(true);
  });
  it("실패·거부는 sticky 로 남고 성공만 사라지는 알림이다(설계 6.2 4항)", () => {
    const f = fnBody("trashPollOrder");
    expect(f.includes('if (severity === "ok") {')).toBe(true);
    const iOk = f.indexOf('toast("feed", "휴지통 삭제 완료"');
    const iBad = f.indexOf("stickyToast(\n      \"trash-order\"");
    expect(iOk >= 0 && iBad > iOk).toBe(true);
  });
  it("★MJR-4: 삭제 실패의 수신증은 순수 모듈이 가르고, 전송 불확실에서는 **단추를 다시 열지 않는다**", () => {
    const f = fnBody("trashDeleteFlow");
    // 판정은 화면이 아니라 순수 함수가 한다(제목·본문·상태·재전송 허용 여부가 한 자리에 있다).
    expect(f.includes('trashTransportReceipt(String(e).split(":")[0], String(e))')).toBe(true);
    expect(f.includes('stickyToast("trash-order", "watchdog", rcpt.title, rcpt.body)')).toBe(true);
    expect(f.includes("trashSetStatus(rcpt.status);")).toBe(true);
    // ★맹목 재전송 차단 — `blocked` 분기는 `trashDeleting` 을 풀지 않고 돌아간다(깃발이 단추를 잠근다).
    const iBlocked = f.indexOf('if (rcpt.resend === "blocked") {');
    const iRelease = f.indexOf("trashDeleting = false;", iBlocked);
    const iReturn = f.indexOf("return;", iBlocked);
    expect(iBlocked >= 0).toBe(true);
    expect(iReturn > iBlocked && (iRelease < 0 || iRelease > iReturn)).toBe(true);
    // 단추를 잠그는 곳은 그 깃발이다(두 벌의 잠금 규칙을 만들지 않는다).
    expect(fnBody("trashSyncDeleteButton").includes("btn.disabled = trashDeleting || !supported || n === 0;")).toBe(true);
    // ⛔화면이 **토스트 제목**을 다시 짓지 않는다: 그 문자열이 이 흐름에 리터럴로 없다
    //   (취소·확인 0건의 「아무 것도 지우지 않았습니다」는 사실 주장이 참인 자리이므로 남는다).
    expect(f.includes("휴지통 삭제 — 지우지 않았습니다")).toBe(false);
    expect(f.includes("결과를 확인할 수 없습니다")).toBe(false);
  });
  it("거부 코드는 순수 모듈의 표로만 문구가 된다(화면이 사유를 다시 지어내지 않는다)", () => {
    // ★자리가 옮겼다: 미리보기의 거부는 이제 **모달 안에서** 문구가 된다(동기 경로 퇴역).
    const m = fnBody("trashConfirmModal");
    expect((m.match(/trashRefusalText\(/g) ?? []).length).toBeGreaterThanOrEqual(3);
    expect(m.includes("String(e).split(\":\")[0]")).toBe(true);
    // 명령(삭제) 쪽은 **수신증 함수**를 거쳐 같은 표에 닿는다 — 흐름이 표를 직접 부르지 않는다
    //   (전송 부류를 가르는 자리를 지나치지 않게 하려고 한 단계를 더 세웠다).
    const d = fnBody("trashDeleteFlow");
    expect(d.includes("trashTransportReceipt(")).toBe(true);
    expect(d.includes("trashRefusalText(")).toBe(false);
  });
});
