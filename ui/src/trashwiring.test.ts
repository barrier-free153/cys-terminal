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
    for (const f of ["refreshTrash", "trashDeleteFlow", "trashPollOrder", "trashStopPoll", "renderTrash"])
      expect({ 함수: f, setInterval: fnBody(f).includes("setInterval(") }).toEqual({ 함수: f, setInterval: false });
    expect(fnBody("trashPollOrder").includes("setTimeout(")).toBe(true);
  });
});

describe("데몬이 부르는 네 명령만 쓰고, 이름은 그대로 돌려보낸다", () => {
  it("invoke 는 trash_list·trash_preview·trash_delete·trash_order 넷뿐이다", () => {
    const block = code.slice(code.indexOf("function trashStopPoll("), code.indexOf("async function refreshBoard("));
    const calls = [...block.matchAll(/invoke\("([a-z_]+)"/g)].map((m) => m[1]);
    expect([...new Set(calls)].sort()).toEqual(["trash_delete", "trash_list", "trash_order", "trash_preview"]);
  });
  it("미리보기·명령은 **고른 이름만** 보내고, 명령은 미리보기가 준 confirm_id·digest 를 되보낸다", () => {
    const f = fnBody("trashDeleteFlow");
    expect(f.includes('invoke("trash_preview", { names })')).toBe(true);
    expect(f.includes("confirmId: prev.confirm_id")).toBe(true);
    expect(f.includes("digest: prev.digest")).toBe(true);
    expect(f.includes("names: confirmNames")).toBe(true);
  });
});

describe("확인 관문 — 클릭만으로 넘어가는 길이 없다", () => {
  const modal = fnBody("trashConfirmModal");
  it("타이핑 확인은 순수 모듈의 판정으로만 열린다(자동교정 차단 · 불일치 사유 표시)", () => {
    expect(modal.includes("trashPhraseMatches(inp.value, names)")).toBe(true);
    expect(modal.includes("yes.disabled = !trashPhraseMatches")).toBe(true);
    expect(modal.includes("trashMismatchHint(inp.value, names)")).toBe(true);
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
    expect(modal.includes(').textContent = lines.join("\\n");')).toBe(true);
    for (const f of ["renderTrash", "renderTrashResult", "renderTrashUnavailable"])
      expect({ 함수: f, innerHTML: fnBody(f).includes("innerHTML") }).toEqual({ 함수: f, innerHTML: false });
    expect(fnBody("renderTrash").includes("name.textContent = e.name;")).toBe(true);
  });
  it("모달이 참을 돌려준 **뒤에만** 명령이 나간다 · 취소는 아무 것도 지우지 않는다고 말한다", () => {
    const f = fnBody("trashDeleteFlow");
    const iModal = f.indexOf("await trashConfirmModal(");
    const iOrder = f.indexOf('invoke("trash_delete"');
    expect(iModal >= 0 && iOrder > iModal).toBe(true);
    expect(f.includes("if (!ok) {")).toBe(true);
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
    const iPrev = f.indexOf('invoke("trash_preview"');
    expect(iStatus >= 0 && iPrev > iStatus).toBe(true);
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
  it("거부 코드는 순수 모듈의 표로만 문구가 된다(화면이 사유를 다시 지어내지 않는다)", () => {
    const f = fnBody("trashDeleteFlow");
    expect((f.match(/trashRefusalText\(/g) ?? []).length).toBeGreaterThanOrEqual(3);
    expect(f.includes("String(e).split(\":\")[0]")).toBe(true);
  });
});
