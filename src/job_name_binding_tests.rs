//! ★A-PRIME-S1 — 비동기 휴지통 미리보기의 **잡 식별자 이름 결박 핀**(테스트 전용 · 프로덕션 코드 0행).
//!
//! 무엇을 못 박는가: 잡 식별자 하나가 **세 다리**를 건너며 격식(case)을 두 번 갈아입는다.
//!   · 다리 1 — 화면 → Tauri 명령 인자: 화면은 `jobId`(camel), Rust 인자는 `job_id`(snake).
//!     갈아입히는 것은 `#[tauri::command]` 매크로다: 인자 격식 기본값이 camelCase 이고
//!     (실측 — `tauri-macros-2.6.2/src/command/wrapper.rs:51` `argument_case: ArgumentCase::Camel`
//!     · 같은 파일 :505·:507 `match attributes.argument_case` / `key = key.to_lower_camel_case()`),
//!     그래서 **Rust 인자 이름의 camel 변환**이 화면이 실어야 하는 키다.
//!     `#[tauri::command(rename_all = "snake_case")]` 를 달면 그 규칙이 뒤집힌다 — 이 핀은
//!     인자 이름만 보지 않고 **그 속성까지 읽어** 기대 키를 계산한다.
//!   · 다리 2 — Tauri → 데몬 RPC: 변환이 **없다**. `json!({"job_id": …})` 의 키와 데몬의
//!     `params.get("job_id")` 가 **바이트로 같아야** 한다.
//!   · 다리 3 — 데몬 응답 → 화면: 역시 변환이 없다(Tauri 는 `resp["result"]` 를 그대로 흘린다).
//!     데몬이 `{"job_id": …}` 로 답하고 화면은 `job.job_id` 로 읽는다.
//!     ★다리 1 과 **격식이 다르다**: 들어갈 때는 camel, 나올 때는 snake 다. 「일관성」을 이유로
//!     어느 한쪽을 고치면 미리보기가 조용히 멈춘다(스피너가 영원히 돌고 거부도 안 뜬다).
//!
//! 왜 선언이 아니라 **대조**인가: 세 다리의 이름을 이 파일에 상수로 적어 두면 그 상수가 또 하나의
//! 사본이 되고, 사본은 어긋난다. 그래서 이 핀은 **양쪽 소스에서 각각 꺼내 비교한다** — 어느 한쪽이
//! 한 글자라도 움직이면 두 값이 달라져 적색이 된다.
//!
//! ★대가 고지: 이 시험은 `ui/src/main.ts` · `src-tauri/src/main.rs` · `src/bin/cysd/trash.rs` 를
//! **런타임에 읽는다**. 그 세 파일을 건드리는 사람이 이 핀을 적색으로 만든다 — 그것이 이 핀의
//! 목적이지만, 대가는 「내 모듈을 건드리지 않은 변경이 lib 스위트에서 적색이 된다」는 것이다.
//! 그래서 실패 메시지는 **무엇을 어떻게 맞추라는 지시**를 담는다(진단할 사람이 남의 자리에 있다).
//! 전제에서 죽는 것과 판별에서 죽는 것을 메시지 접두 `[전제]`/`[판별]` 로 가른다 —
//! 전제(파일을 못 읽었다·호출 자리를 못 찾았다)에서 죽은 적색은 **판별력의 증명이 아니다**.
//! ⚠그리고 `[판별]` 라벨 **자체도** 의미적 특이성의 증거가 아니다 — 종전 판은 속성 **순서**만 바뀐
//! 객체에 `[판별]` 적색을 냈다(codex R1 X6). 라벨은 **의도**이고, 증거는 **반증 변이**다.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! ★**보장 범위 — 무엇을 보장하고 무엇을 보장하지 않는가**(2026-10-14 · codex R1 N1~N4 **2차** 수리 뒤)
//! ─────────────────────────────────────────────────────────────────────────────
//! **보장한다**(이 세 시험이 적색이 되는 것으로):
//!  ⒜ 다리 1 — 화면 invoke 인자 객체에 **기대 키가 실제 속성으로 있다**. 기대 키는 Tauri 인자 이름 +
//!     그 함수에 붙은 속성이 정한 격식으로 **계산**한다. 속성은 **토큰으로 파싱**하므로 공백·줄바꿈
//!     차이에 흔들리지 않는다(`rename_all="snake_case"` 와 `rename_all =  "snake_case"` 를 **같게** 본다).
//!     ★키는 **문자열 리터럴과 괄호 깊이를 추적해** 최상위 쉼표·콜론만 경계로 세어 꺼낸다 —
//!     문자열 **값** 안의 `, k: v` 를 속성으로 **오인하지 않는다**(codex R1 N1 이 실증한 자리 ·
//!     그 꼴은 이제 기대 키가 없는 것으로 읽혀 `[판별]` 적색이다).
//!  ⒝ 다리 2 — Tauri 가 보내는 RPC params 키와 데몬이 `params.get` 으로 읽는 키가 **바이트로 같다**.
//!  ⒞ 다리 3 — 데몬 성공 응답 키와 화면의 **타입 주석** 칸, 그리고 화면이 **실제로 읽는 두 자리**가
//!     **종류별로 하나씩**(옵셔널 가드 `job?.<칸>` 하나 · 폴링 전달 `poll(job.<칸>)` 하나) 있고
//!     그 이름이 **식별자 경계까지** 같다. ★개수는 **합계가 아니라 종류별로** 센다 — 합계는 한 집단이
//!     다른 집단의 관측 의무를 대신하게 만든다(codex R1 N3: 가드 2 + 인식된 폴링 0 이 통과했다).
//!     식별자 경계는 `[A-Za-z0-9_$]`(ASCII 근사)이고, 접두와 이름 사이의 **공백·줄바꿈은 지원한다**
//!     (`job?. job_id` 는 같은 칸을 읽는 정당한 서식이므로 초록이다 — codex R1 Q3).
//!  ⒟ 보고 문면의 **행 번호가 그 줄의 문면과 맞다**(찍은 줄을 파일에서 다시 읽어 대조한다).
//!     ★그 **거부 경로에 직접 부정 검체가 있다**(`cite_rejects_…` 시험): 그 줄에 없는 바늘·한 줄
//!     어긋난 바늘을 주면 거부하는지 **관찰한다**. codex R1 N4 가 실증한 구멍이 이것이다 — 종전
//!     25 셀은 이 단언의 **제거를 구별하지 못했다** — 내 재측정(2026-10-14): 그 25 셀에 codex 의 추가
//!     7 셀을 더한 **32 셀 전수**에서 제거본과 원본의 rc·실패 시험 이름·`[전제]`/`[판별]`·찍힌 행
//!     번호가 **차이 0** 이었다. 이 검체를 더한 뒤 같은 제거는 **32/32 셀에서 구별된다**.
//! **보장하지 않는다**(적색이 되지 않는 것 · 거짓 안심을 막기 위해 적는다):
//!  ⓵ **미리보기가 돈다**는 것 — 이 핀은 **이름의 맞물림**만 본다(동작은 데몬 쪽 왕복 검체의 몫).
//!  ⓶ **TypeScript 컴파일·런타임** — 여기서 적색이 아니어도 `tsc`·실행이 거부할 수 있고 그 반대도 있다.
//!  ⓷ **모형 밖의 문법**: `invoke` 인자의 전개(`...x`)·계산 키(`[k]`)·**키 자리의** 문자열(`"k": v`)·
//!     키 자리의 호출 · 닫히지 않은 리터럴·괄호 · 중첩 중괄호 · 속성의 `rename`·`root`·알 수 없는 키·
//!     비문자열 값·중복 `rename_all`·중첩 괄호·**빈 항목만 있는 꼴**(`#[tauri::command(,,,)]`) ·
//!     `pub fn`·매크로가 만든 명령 정의. 이 꼴들은 **통과시키지 않고 `[전제]` 로 멈춘다**
//!     (모르는 문법을 camel 로 **가정하지 않는다**).
//!     ★반대로 **값 자리의 문자열은 모형 안이다**: `{ jobId, mode: "fast" }` 는 초록이다(쉼표·콜론이
//!     문자열 안에 있어도 경계로 세지 않는다). 모형을 좁히면서 **정당한 코드에 적색을 내지 않기 위해**
//!     거부가 아니라 **파싱**을 골랐다.
//!  ⓸ **유니코드 식별자**: 화면이 읽는 칸 이름이 비ASCII 로 이어지면(`job?.job_idé`) 이 핀은 그것을
//!     ASCII 부분으로 **잘라 승인하지 않고** `[전제]` 로 멈춘다 — 즉 **유니코드 식별자를 지원하지
//!     않는다**(codex R1 N2: 종전 판은 `job_id` 로 잘라 통과시켰다). 그 꼴이 정당한 코드가 되면
//!     이 핀을 먼저 넓혀야 한다.
//!  ⓹ **별칭·값 흐름**: 이 핀은 **키 이름**을 본다. `let jid = job_id.clone();` 처럼 값이 다른
//!     이름으로 흐르는 경로는 세지 않는다(codex R1 이 census 한계로 지적한 자리와 같다).
//!  ⓺ **인자가 여럿인 경우의 나머지 키**: 기대 키의 **존재**를 보므로 다른 키가 더 있어도 초록이다
//!     (순서도 무관하다). 그 선택은 의도다 — 「UI 가 인자를 하나 더 늘리는 정당한 변경」에서
//!     적색이 나면 그 게이트는 **옳은 코드에 발화하는** 게이트가 된다.
//!  ⓻ **이 축의 다른 칸**: `status`·`progress`·`result`·`refusal`·`progress.elapsed_secs` 는
//!     같은 변환 없는 경계를 건너지만 **묶여 있지 않다**(백로그 B-193).
//!  ⓼ **윈도우 레인**: `windows-health.yml` 의 `cargo test --lib` 는 **필터형**이라 이 모듈이 돌지
//!     않는다. 도는 자리는 macOS 의 태그 레인(`release.yml`)과 브랜치 레인(`ci-branch.yml`)이다.
#![cfg(test)]

use std::path::{Path, PathBuf};

const UI: &str = "ui/src/main.ts";
const SHIM: &str = "src-tauri/src/main.rs";
const DAEMON: &str = "src/bin/cysd/trash.rs";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("[전제] {rel} 를 읽지 못했다({e}) — 측정 불능은 통과가 아니다"))
}

/// `[a-z][a-z0-9_]*` 만 받는 좁은 camel 변환(heck 의 전체 알고리즘이 아니다 — 인자 이름이
/// 이 모양인지 **먼저 단언**하고 쓴다. 숫자·대문자가 섞인 이름이 오면 전제에서 멈춘다).
fn to_lower_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut up = false;
    for c in s.chars() {
        if c == '_' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn is_snake_ident(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_js_ident(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// ★**1-기반 행 번호 = 그 바이트 앞의 개행 수 + 1**(N4 수리).
/// 종전 판은 `src[..byte].lines().count()` 였다 — 그것은 「**완성된 줄**의 수」라서 바이트가
/// **열 0**(줄 머리)이면 **한 줄 앞**을 돌려준다. codex R1 N4 가 그것을 실증했다: 실패 문면이
/// Tauri 서명을 `main.rs:5037`(= `#[tauri::command]` 속성 줄)로 찍었고 함수는 `:5038` 이다.
/// 줄 중간 바이트(화면 쪽 인용)에서는 두 정의가 같은 값을 내므로 **그 자리만 보고는 안 보인다**.
/// ⛔**고정 오프셋으로 고치지 않는다** — 반대 방향 오류를 만든다(codex 명시). 대신 아래 `cite()` 가
/// 찍은 줄의 **문면을 파일에서 다시 읽어 대조**한다.
fn line_of(src: &str, byte: usize) -> usize {
    src[..byte].matches('\n').count() + 1
}

/// 인용 자리를 **문면까지 대조해서** 돌려준다 — (1-기반 행 번호, 그 줄 전문).
/// ★`needle` 이 그 줄에 실제로 있어야 한다. 없으면 **[전제]** 에서 멈춘다 ⇒ 행 계수가 어긋나면
/// 조용히 틀린 번호를 찍는 대신 **측정 불능**으로 선다. 이것이 N4 의 수리다(오프셋이 아니라 대조).
fn cite(src: &str, byte: usize, needle: &str, rel: &str) -> usize {
    let n = line_of(src, byte);
    let text = src.lines().nth(n - 1).unwrap_or("");
    assert!(
        text.contains(needle),
        "[전제] {rel}:{n} 을 인용하려는데 그 줄에 {needle:?} 가 없다(그 줄: {text:?}) — \
         행 계수가 어긋났다. ⛔고정 오프셋을 더하지 말고 계수 정의를 고쳐라"
    );
    n
}

/// `<접두>` 뒤에 붙은 **완전한 멤버 식별자**를 전수로 꺼낸다 — (식별자, 바이트 위치)(N2 수리).
///
/// 종전 판은 `ui.contains("job?.job_id")` 하나였다. 그래서 `job?.job_idd` 처럼 **접미가 한 글자
/// 붙어도** 기대 문자열이 **부분문자열로 남아** 세 시험 전부 통과했다(codex X4 실증 — 「읽는 이름이
/// 실제로 같다」를 **증명하지 못한 것**이다). 이제 식별자 **경계까지** 읽어 전수로 모은 뒤
/// **완전 비교**한다. 식별자 글자는 `[A-Za-z0-9_$]`(TS 식별자의 좁은 근사)로 본다.
///
/// ★2차 수리(codex R1 N2) — 좁은 근사의 **경계 밖을 잘라 승인하던 것**을 닫았다:
///  ⑴ 접두와 이름 사이의 **공백·줄바꿈을 지원한다**. 종전 판은 `job?. job_id` 를 **빈 이름**으로 읽어
///     「이름이 다르다」는 `[판별]` 적색을 냈다 — 이름은 **같았으므로** 그 라벨은 사실과 달랐다
///     (codex Q3). 라벨이 거짓이면 그 적색은 진단이 아니라 오도다.
///  ⑵ 이름을 하나도 읽지 못하면(빈 이름) `[전제]` 로 멈춘다 — 파서의 무능을 이름 불일치로 적지 않는다.
///  ⑶ 경계 글자가 **비ASCII** 면 그 이어짐이 실제 식별자의 일부일 수 있으므로(TS 식별자는 유니코드를
///     받는다) **잘라서 승인하지 않고** `[전제]` 로 멈춘다. 종전 판은 `job?.job_idé` 를 `job_id` 로
///     잘라 세 시험 전부 통과시켰다(codex Q2) — 「읽는 이름이 실제로 같다」는 보증의 반례다.
fn member_reads(src: &str, prefix: &str, rel: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(i) = src[from..].find(prefix) {
        let at = from + i;
        let rest = &src[at + prefix.len()..];
        let ws = rest.len() - rest.trim_start_matches([' ', '\t', '\r', '\n']).len();
        let tail = &rest[ws..];
        let end = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
            .unwrap_or(tail.len());
        let name = &tail[..end];
        assert!(
            !name.is_empty(),
            "[전제] {rel}:{} 의 `{prefix}` 뒤에서 식별자를 읽지 못했다(그 뒤 24바이트: {:?}) — \
             이 핀이 모형화하는 것은 `{prefix}<식별자>`(사이의 공백·줄바꿈 허용)뿐이다. \
             ⛔읽지 못한 것을 **이름 불일치로 적지 않는다**(측정 불능은 통과도 판별도 아니다)",
            line_of(src, at),
            &tail[..tail.len().min(24)]
        );
        if let Some(b) = tail[end..].chars().next() {
            assert!(
                b.is_ascii(),
                "[전제] {rel}:{} 의 `{prefix}{name}` 바로 뒤에 비ASCII 글자 {b:?} 가 **이어진다** — \
                 이 핀의 식별자 경계는 `[A-Za-z0-9_$]`(ASCII 근사)이므로 내가 읽은 {name:?} 가 실제 \
                 식별자라는 보증이 성립하지 않는다. ⛔잘라서 승인하지 않는다(유니코드 식별자를 \
                 지원하려면 이 핀의 경계 정의를 먼저 넓혀라)",
                line_of(src, at)
            );
        }
        out.push((name.to_string(), at + prefix.len() + ws));
        from = at + prefix.len();
    }
    out
}

/// 중첩 없는 **객체 리터럴 본문**에서 최상위 키를 순서대로 꺼낸다(N1 수리 · codex R1).
///
/// 종전 판은 `inside.split(',')` 뒤 `part.split(':').next()` 였다 — 쉼표·콜론을 **문자열 안에서도**
/// 경계로 셌다. 그래서 `{ unused: "x, jobId: y" }` 의 문자열 **값** 안에서 `jobId` 를 키로 꺼내
/// 「기대 키가 있다」로 **승인**했다(codex R1 N1 실증 — 실제 객체에 그 키는 **없다**. 이것은 보장 ⒜
/// 문면을 거짓으로 만드는 **키 존재 오판**이었다).
/// 이제 문자열 리터럴(`"` `'` `` ` ``)과 괄호(`(` `[`) 깊이를 추적해 **최상위 쉼표·콜론만** 경계로 센다.
/// ⛔**키 자리**(최상위 콜론 앞)에 문자열·계산 키·호출이 오면 `[전제]` 로 거부한다 — 모르는 꼴을
/// 키로 **가정하지 않는다**. 값 자리의 문자열은 **지원한다**(정당한 코드에 적색을 내지 않기 위해).
fn object_literal_keys(inside: &str, what: &str) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    let mut key = String::new();
    let mut after_colon = false;
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut chars = inside.chars();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == '\\' {
                chars.next(); // 이스케이프된 한 글자는 문자열의 내용이다
            } else if c == q {
                quote = None;
            }
            continue; // ★문자열 안의 쉼표·콜론은 경계가 **아니다**(N1 의 핵심)
        }
        match c {
            '"' | '\'' | '`' => {
                assert!(
                    after_colon || depth > 0,
                    "[전제] {what}: 키 자리에 문자열 리터럴이 있다 — 문자열 키(`\"k\": v`)는 이 핀의 \
                     모형 밖이다(값 자리의 문자열은 모형 안이다)"
                );
                quote = Some(c);
            }
            '(' | '[' => {
                assert!(
                    after_colon,
                    "[전제] {what}: 키 자리에 {c:?} 가 있다 — 계산 키(`[k]`)·호출은 이 핀의 모형 밖이다"
                );
                depth += 1;
            }
            ')' | ']' => {
                assert!(depth > 0, "[전제] {what}: 닫는 {c:?} 의 짝이 없다 — 이 핀의 꺼내기 규약이 깨졌다");
                depth -= 1;
            }
            ',' if depth == 0 => {
                let k = key.trim().to_string();
                key.clear();
                after_colon = false;
                if !k.is_empty() {
                    keys.push(k);
                }
            }
            ':' if depth == 0 => after_colon = true,
            _ => {
                if !after_colon {
                    key.push(c);
                }
            }
        }
    }
    assert!(quote.is_none(), "[전제] {what}: 문자열 리터럴이 닫히지 않았다 — 경계를 셀 수 없다");
    assert_eq!(depth, 0, "[전제] {what}: 괄호가 닫히지 않았다 — 경계를 셀 수 없다");
    let k = key.trim().to_string();
    if !k.is_empty() {
        keys.push(k);
    }
    keys
}

/// `{ … }` 한 쌍을 꺼낸다(중첩 없음 전제 — 중첩이면 전제에서 멈춘다).
fn brace_span(src: &str, from: usize, what: &str) -> (usize, usize) {
    let open = src[from..]
        .find('{')
        .unwrap_or_else(|| panic!("[전제] {what}: 여는 중괄호를 찾지 못했다"))
        + from;
    let close = src[open + 1..]
        .find('}')
        .unwrap_or_else(|| panic!("[전제] {what}: 닫는 중괄호를 찾지 못했다"))
        + open
        + 1;
    assert!(
        !src[open + 1..close].contains('{'),
        "[전제] {what}: 중괄호가 중첩돼 있다 — 이 핀의 꺼내기 규약(중첩 없는 인자 객체 하나)이 깨졌다"
    );
    (open, close)
}

/// 화면의 `invoke("<cmd>", { … })` 인자 객체에서 **키를 전부** 꺼낸다(N3 수리).
/// 종전 판은 **콤마로 쪼갠 첫 키**만 비교했다 — 그래서 `{ jobId, unused: 0 }` 는 통과하고
/// `{ unused: 0, jobId }` 는 **같은 jobId 가 있는데도** `[판별]` 적색이 됐다(codex X5·X6 실증).
/// ★그 적색은 **이름 불일치가 아니라 속성 순서**에서 나온 것이므로 `[판별]` 라벨이 의미적
/// 특이성의 증거가 되지 못했다. 이제 **순서와 무관하게 바인딩으로** 찾는다.
/// ★1글자 어긋난 이름도 꺼내야 한다 — 꺼내지 못하면 판별이 아니라 전제에서 죽고, 그 적색은
/// 판별력의 증명이 못 된다. 그래서 식별자 모양이면 무엇이든 꺼낸다(이름을 안다고 가정하지 않는다).
fn ui_invoke_arg_keys(src: &str, cmd: &str) -> (Vec<String>, usize) {
    let needle = format!("invoke(\"{cmd}\"");
    let n = src.matches(needle.as_str()).count();
    assert_eq!(
        n, 1,
        "[전제] {UI} 의 `{needle}…)` 호출 자리가 {n} 곳이다 — 이 핀은 **한 곳**을 계약으로 삼는다. \
         자리를 늘렸다면 이 핀도 함께 늘려라(첫 자리만 보는 핀은 두 번째 자리의 오타를 못 본다)"
    );
    let at = src.find(needle.as_str()).expect("count==1 이면 존재한다");
    let (open, close) = brace_span(src, at, &format!("{UI} invoke(\"{cmd}\") 인자 객체"));
    let inside = &src[open + 1..close];
    let line = cite(src, at, &needle, UI);
    // ★문자열·괄호를 인식해 **최상위** 경계로만 쪼갠다(N1 수리) — 종전 판은 문자열 값 안의
    //   `, jobId: y` 를 속성으로 꺼내 **없는 키를 있다고** 승인했다(codex R1 N1).
    let keys = object_literal_keys(inside, &format!("{UI}:{line} invoke(\"{cmd}\") 인자 객체"));
    for k in &keys {
        assert!(
            is_js_ident(k),
            "[전제] {UI}:{line} 의 invoke 인자 객체에서 키를 식별자로 꺼내지 못했다(꺼낸 것: {k:?}) — \
             전개(`...x`)·계산 키(`[k]`)·문자열 키 같은 꼴은 이 핀이 모형화하지 않는다"
        );
    }
    assert!(
        !keys.is_empty(),
        "[전제] {UI}:{line} 의 invoke 인자 객체에서 키를 하나도 꺼내지 못했다"
    );
    (keys, line)
}

/// 열 0 에 있는 **최상위 정의**의 위치를 돌려준다 — 시험 코드가 인용하는 같은 문자열(들여쓰기된
/// 문자열 리터럴)을 정의로 세지 않는다.
///
/// ★이 보조 함수는 **대조 실행이 만들어 준 것**이다: `fn trash_preview_poll(` 를 그냥 센 첫 판은
/// `src-tauri/src/main.rs` 의 **명령 등재 핀이 인용하는 문자열 리터럴**까지 세어 「정의가 2 곳」으로
/// 전제에서 죽었다. 전제에서 죽는 적색은 판별력의 증명이 못 되므로, 세는 자리를 좁혔다.
/// 한계(정직 고지): `pub fn`·매크로가 만든 정의는 이 바늘이 못 본다 — 그 꼴이 생기면 개수가
/// 어긋나 **전제에서** 멈춘다(조용히 통과하지는 않는다).
fn top_level_fn(src: &str, name: &str, rel: &str) -> usize {
    let plain = format!("\nfn {name}(");
    let asy = format!("\nasync fn {name}(");
    let n = src.matches(plain.as_str()).count() + src.matches(asy.as_str()).count();
    assert_eq!(
        n, 1,
        "[전제] {rel} 의 **최상위**(열 0) `fn {name}(` 정의가 {n} 곳이다 — 하나여야 한다. \
         0 이면 이름이 바뀌었거나 `pub`·매크로 꼴이 됐다(이 핀의 바늘을 함께 고쳐라), \
         2 이상이면 이 핀이 무엇을 대조하는지 알 수 없다"
    );
    src.find(asy.as_str())
        .or_else(|| src.find(plain.as_str()))
        .map(|i| i + 1)
        .expect("개수 단언이 통과했으면 존재한다")
}

/// `#[tauri::command…]` 속성에서 **인자 격식**을 토큰으로 파싱한다 — camel 이면 `true`(N1 수리).
///
/// 종전 판은 `attr.contains("rename_all = \"snake_case\"")` **하나**로 분류했다. 그래서 공백이
/// 0 개(`rename_all="snake_case"`)거나 2 개(`rename_all =  "snake_case"`)인 **같은 뜻의 Rust 표기**가
/// 전부 「camel」로 분류돼 **세 시험 전부 통과**했다(codex X1·X2 실증 — 핀이 `jobId` 를 snake 기대에
/// 대해 승인한 것이다). 공백은 Rust 속성의 의미가 아니다.
///
/// ★지원 문법(이 핀이 **모형화하는** 것): 인자 없음 · `async` · `rename_all = "camelCase"` ·
/// `rename_all = "snake_case"`(공백·줄바꿈 임의).
/// ⛔그 밖의 표기는 **[전제] 로 거부**한다 — `rename = "…"`(명령 **이름**을 바꾼다 ⇒ 화면의
/// `invoke("<이름>")` 과의 랑데부가 이 핀의 모형 밖으로 나간다) · `root = "…"` · 알 수 없는 키 ·
/// 비문자열 값 · 중복 `rename_all` · 중첩 괄호. ★**모르는 문법을 camel 로 가정하지 않는다** —
/// 그 가정이 X1·X2 를 통과시킨 바로 그 기제다. 거부는 판별이 아니라 **측정 불능**이므로 `[전제]` 다.
fn tauri_arg_case_is_camel(attr: &str, rel: &str, cmd: &str) -> bool {
    const HEAD: &str = "#[tauri::command";
    let body = attr
        .strip_prefix(HEAD)
        .unwrap_or_else(|| panic!("[전제] {rel} {cmd}: 속성이 `{HEAD}` 로 시작하지 않는다({attr:?})"))
        .trim_end()
        .strip_suffix(']')
        .unwrap_or_else(|| panic!("[전제] {rel} {cmd}: 속성이 `]` 로 닫히지 않는다({attr:?})"))
        .trim();
    if body.is_empty() {
        return true; // `#[tauri::command]` — 기본값 camelCase (tauri-macros 2.6.2 wrapper.rs:51)
    }
    let inner = body
        .strip_prefix('(')
        .and_then(|b| b.strip_suffix(')'))
        .unwrap_or_else(|| {
            panic!("[전제] {rel} {cmd}: 속성 인자를 `(…)` 로 읽지 못했다({attr:?}) — 이 핀의 모형 밖이다")
        });
    assert!(
        !inner.contains('(') && !inner.contains('[') && !inner.contains('{'),
        "[전제] {rel} {cmd}: 속성 인자에 중첩 괄호가 있다({inner:?}) — 이 핀은 평평한 인자만 모형화한다"
    );
    // ★빈 항목만 있는 꼴(`#[tauri::command(,,,)]`)을 **기본값으로 가정하지 않는다**(codex R1 Q6).
    //   `(…)` 가 비어 있는 것(`#[tauri::command()]`)은 정당한 Rust 이므로 기본값(camel)이다.
    if inner.trim().is_empty() {
        return true;
    }
    let mut readable = 0usize;
    let mut case: Option<bool> = None;
    for item in inner.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        readable += 1;
        if item == "async" {
            continue;
        }
        let (k, v) = item.split_once('=').unwrap_or_else(|| {
            panic!(
                "[전제] {rel} {cmd}: 속성 인자 {item:?} 를 `키 = 값` 으로 읽지 못했다 — \
                 이 핀이 모형화하는 것은 인자 없음·`async`·`rename_all = \"camelCase\"|\"snake_case\"` 뿐이다"
            )
        });
        let (k, v) = (k.trim(), v.trim());
        assert_eq!(
            k, "rename_all",
            "[전제] {rel} {cmd}: 모르는 속성 키 {k:?}(값 {v:?}) — ⛔camel 로 **가정하지 않는다**. \
             `rename` 은 명령 **이름**을 바꾸므로 화면의 invoke 문자열과의 랑데부가 이 핀의 모형 밖으로 \
             나간다. 이 핀을 그 문법까지 넓히고 나서 쓰라"
        );
        let want = match v {
            "\"camelCase\"" => true,
            "\"snake_case\"" => false,
            other => panic!(
                "[전제] {rel} {cmd}: `rename_all` 값 {other:?} 는 이 핀의 모형 밖이다 \
                 (tauri-macros 2.6.2 wrapper.rs:69·:70 이 받는 값은 \"camelCase\"·\"snake_case\" 둘뿐이다)"
            ),
        };
        assert!(
            case.is_none(),
            "[전제] {rel} {cmd}: `rename_all` 이 두 번 적혔다 — 어느 쪽이 이기는지 이 핀은 모른다"
        );
        case = Some(want);
    }
    assert!(
        readable > 0,
        "[전제] {rel} {cmd}: 속성 인자 {inner:?} 에서 읽을 항목이 하나도 없다 — 빈 항목만 있는 꼴을 \
         **기본값(camelCase)으로 가정하지 않는다**(모르는 문법은 거부가 맞다)"
    );
    case.unwrap_or(true)
}

/// Tauri 명령의 **인자 격식**과 **첫 인자 이름**을 꺼낸다.
/// 격식은 그 함수에 **실제로 붙은** `#[tauri::command…]` 속성에서 읽는다(붙어 있음을 증명한다 —
/// 남의 함수의 속성을 읽어 오면 그 판정은 거짓이다).
fn tauri_command_first_arg(src: &str, cmd: &str) -> (bool /*camel*/, String, usize) {
    let sig = format!("fn {cmd}(");
    let at = top_level_fn(src, cmd, SHIM);
    let attr_at = src[..at]
        .rfind("#[tauri::command")
        .unwrap_or_else(|| panic!("[전제] {SHIM} 의 `{sig}` 앞에 `#[tauri::command` 속성이 없다"));
    let attr_end = src[attr_at..]
        .find(']')
        .unwrap_or_else(|| panic!("[전제] {SHIM}: `#[tauri::command` 속성이 닫히지 않았다"))
        + attr_at;
    // 속성이 **이 함수에** 붙어 있는가: 속성과 서명 사이에 공백·`async` 외의 것이 없어야 한다.
    let between: String = src[attr_end + 1..at].chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        between.is_empty() || between == "async" || between == "pub" || between == "pubasync",
        "[전제] {SHIM}: `#[tauri::command…]` 와 `{sig}` 사이에 다른 것이 있다({between:?}) — \
         이 속성이 그 함수의 것이라는 증명이 성립하지 않는다"
    );
    let attr = &src[attr_at..=attr_end];
    let camel = tauri_arg_case_is_camel(attr, SHIM, cmd);
    let args_open = src[at..]
        .find('(')
        .unwrap_or_else(|| panic!("[전제] {SHIM}: `{sig}` 인자 목록이 열리지 않았다"))
        + at
        + 1;
    let args_close = src[args_open..]
        .find(')')
        .unwrap_or_else(|| panic!("[전제] {SHIM}: `{sig}` 인자 목록이 닫히지 않았다"))
        + args_open;
    let first = src[args_open..args_close]
        .split(',')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    assert!(
        is_snake_ident(&first),
        "[전제] {SHIM}:{} 의 `{sig}` 첫 인자 이름을 `[a-z][a-z0-9_]*` 로 꺼내지 못했다(꺼낸 것: {first:?}) — \
         이 핀의 camel 변환기는 그 모양만 받는다(heck 의 전체 알고리즘이 아니다)",
        line_of(src, at)
    );
    (camel, first, cite(src, at, &sig, SHIM))
}

/// **최상위** 함수의 본문(서명 뒤 첫 `{` 부터 열 0 의 `}` 까지)을 꺼낸다.
fn fn_body<'a>(src: &'a str, name: &str, rel: &str) -> &'a str {
    let at = top_level_fn(src, name, rel);
    let open = src[at..]
        .find('{')
        .unwrap_or_else(|| panic!("[전제] {rel} {name}: 본문 여는 중괄호가 없다"))
        + at;
    let end = src[open..]
        .find("\n}\n")
        .unwrap_or_else(|| panic!("[전제] {rel} {name}: 본문 닫는 중괄호(열 0)를 찾지 못했다"))
        + open;
    &src[open..end]
}

/// `json!({ "k": v, … })` 의 (키, 값) 쌍을 순서대로 꺼낸다.
fn json_macro_pairs(body: &str, what: &str) -> Vec<(String, String)> {
    let at = body
        .find("json!({")
        .unwrap_or_else(|| panic!("[전제] {what}: `json!({{` 를 찾지 못했다"));
    let start = at + "json!({".len();
    let close = body[start..]
        .find("})")
        .unwrap_or_else(|| panic!("[전제] {what}: `json!({{…}})` 가 닫히지 않았다"))
        + start;
    let inner = &body[start..close];
    let mut out = Vec::new();
    for part in inner.split(',') {
        let mut it = part.splitn(2, ':');
        let k = it.next().unwrap_or("").trim().trim_matches('"').to_string();
        let v = it.next().unwrap_or("").trim().to_string();
        if !k.is_empty() && !v.is_empty() {
            out.push((k, v));
        }
    }
    assert!(!out.is_empty(), "[전제] {what}: `json!` 에서 쌍을 하나도 꺼내지 못했다");
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 다리 1 — 화면 → Tauri 명령 인자 (갈아입는 자리 · 변환 규칙까지 함께 본다)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn leg1_screen_invoke_key_matches_the_tauri_command_argument() {
    let ui = read(UI);
    let shim = read(SHIM);
    let (ui_keys, ui_line) = ui_invoke_arg_keys(&ui, "trash_preview_poll");
    let (camel, arg, shim_line) = tauri_command_first_arg(&shim, "trash_preview_poll");
    let expect = if camel { to_lower_camel(&arg) } else { arg.clone() };
    // ★**바인딩으로** 찾는다 — 속성 **순서**가 아니다(N3 수리). 기대 키가 그 객체 안에 **있는가**를
    //   묻는다. 그래서 `{ jobId, unused: 0 }` 와 `{ unused: 0, jobId }` 는 **둘 다 초록**이다
    //   (둘 다 이름 불일치가 없다 · codex X5·X6). 종전 판은 뒤쪽을 `[판별]` 적색으로 떨어뜨렸고,
    //   그것은 이름이 아니라 순서를 잡은 **가짜 적색**이었다.
    assert!(
        ui_keys.iter().any(|k| *k == expect),
        "[판별] 잡 식별자 이름이 다리 1 에서 어긋났다.\n\
         · 화면({UI}:{ui_line}) 이 싣는 키 전수: {ui_keys:?}\n\
         · Tauri 명령({SHIM}:{shim_line}) 의 인자: {arg:?} · 인자 격식: {}\n\
         · 그 둘이 맞으려면 화면 인자 객체에 키 {expect:?} 가 **있어야** 한다(순서는 무관하다).\n\
         고치는 자리는 **하나**다: 화면 키를 {expect:?} 로 되돌리거나, Rust 인자 이름을 바꿨다면 \
         화면 키도 그 camel 변환으로 함께 바꿔라. 어긋나면 Tauri 가 인자를 찾지 못해 미리보기 폴링이 \
         첫 호출에서 거부로 떨어진다(사용자가 보는 것: 확인 창이 즉시 사유 없이 닫히거나 스피너가 멈춘다).",
        if camel { "camelCase(기본값)" } else { "snake_case(rename_all 로 뒤집힘)" }
    );
}

/// ★**계기 자기검사**(N4 수리의 증명 · 고정 오프셋이 아님을 이 시험이 지킨다).
/// `line_of` 를 **열 0** 과 **줄 중간** 두 자리에서 재고, 실제 세 파일에서도 **문면으로** 대조한다.
#[test]
fn line_of_is_one_based_at_column_zero_and_mid_line() {
    // ⒜ 합성 문자열 — 경계를 손으로 센다.
    let t = "a\nbb\nccc\n";
    assert_eq!(line_of(t, 0), 1, "[판별] 파일 첫 바이트는 1행이다");
    assert_eq!(line_of(t, 1), 1, "[판별] 1행의 개행 **앞**은 아직 1행이다");
    assert_eq!(line_of(t, 2), 2, "[판별] 개행 **뒤**(열 0)는 2행이다 — 종전 판이 1 을 돌려줬다");
    assert_eq!(line_of(t, 3), 2, "[판별] 2행 줄 중간은 2행이다");
    assert_eq!(line_of(t, 5), 3, "[판별] 3행 열 0 은 3행이다");
    // ⒝ 실제 파일 — 열 0 자리(Tauri 서명)와 줄 중간 자리(화면 invoke)를 **문면으로** 대조한다.
    //    ★이것이 codex N4 가 실증한 자리다: 종전 판은 서명을 `5037`(속성 줄)로 찍었다.
    let shim = read(SHIM);
    let sig = "async fn trash_preview_poll(";
    let at = shim
        .find(&format!("\n{sig}"))
        .unwrap_or_else(|| panic!("[전제] {SHIM} 에 열 0 의 `{sig}` 가 없다"))
        + 1;
    let n = line_of(&shim, at);
    assert!(
        shim.lines().nth(n - 1).unwrap_or("").contains(sig),
        "[판별] 열 0 바이트의 행 번호가 그 줄 문면과 맞지 않는다(찍은 행 {n}: {:?})",
        shim.lines().nth(n - 1).unwrap_or("")
    );
    // ★종전 정의(`lines().count()`)가 **바로 한 줄 앞**을 돌려주는 자리임을 그 자체로 못 박는다.
    //   ⚠처음에는 「앞 줄이 `#[tauri::command` 줄이어야 한다」로 적었는데, 그것은 속성을 **여러 줄로**
    //   쓴 **정당한 서식**(`#[tauri::command(\n    rename_all = …\n)]`)에서 적색이 된다 — 내 반증 셀
    //   C1f 가 그것을 잡았다. 옳은 코드에 발화하는 단언은 핀이 아니라 고장이므로 **서식에 독립인
    //   형태**로 바꿨다: 「종전 정의는 n-1 을 돌려준다」가 이 검체가 말하려는 전부다.
    assert_eq!(
        shim[..at].lines().count(),
        n - 1,
        "[전제] 열 0 자리에서 종전 정의(`lines().count()`)가 새 정의보다 **정확히 1 작아야** 한다 — \
         이 검체가 지키는 N4 의 모양이 그것이다(그 차이가 없으면 이 자리는 더 이상 열 0 이 아니다)"
    );
    // 속성은 서명 **위**에 있다(몇 줄 위든 무관 — 서식에 독립이다).
    let attr_line = line_of(&shim, shim[..at].rfind("#[tauri::command").unwrap_or_else(|| {
        panic!("[전제] {SHIM}: 서명 위에 `#[tauri::command` 가 없다")
    }));
    assert!(
        attr_line < n,
        "[전제] 속성 줄({attr_line})이 서명 줄({n}) 위에 있어야 한다"
    );
    let ui = read(UI);
    let mid = ui
        .find("invoke(\"trash_preview_poll\"")
        .unwrap_or_else(|| panic!("[전제] {UI} 에 폴링 invoke 가 없다"));
    let m = line_of(&ui, mid);
    assert!(
        ui.lines().nth(m - 1).unwrap_or("").contains("invoke(\"trash_preview_poll\""),
        "[판별] 줄 중간 바이트의 행 번호가 그 줄 문면과 맞지 않는다(찍은 행 {m})"
    );
    // ⒞ 두 자리가 **같은 정의**로 맞는지 — 한쪽만 맞는 오프셋 수리를 이 단언이 막는다.
    assert!(
        line_of(&shim, at) == n && line_of(&ui, mid) == m,
        "[판별] 같은 계수 정의가 두 자리에서 다르게 동작한다 — 고정 오프셋 수리의 징후다"
    );
}

/// ★**`cite` 의 거부 경로 직접 검체**(N4 2차 수리 · codex R1).
///
/// codex R1 N4 가 실증한 것: `cite` 의 문면 대조 단언(`text.contains(needle)`)을 **제거해도**
/// 기존 25 셀의 rc·실패 시험 이름·`[전제]`/`[판별]` 라벨이 **전부 같았다**(차이 0). 즉 그 거부
/// 경로에는 **하중이 걸린 검체가 없었다** — 수리는 돌았지만 **시험된 적이 없는 수리**였다.
/// 그래서 그 경로를 **직접** 두 방향으로 관찰한다:
///  ⒜ 양성 — 바늘이 그 줄에 있으면 1-기반 행 번호를 돌려준다.
///  ⒝ 음성(부정 대조군) — 그 줄에 **없는** 바늘을 주면 거부한다(패닉).
///  ⒞ 음성(한 줄 어긋남) — **이웃 줄**의 바늘을 주면 거부한다. ★이것이 N4 의 원래 고장 모양이다
///     (행 계수가 1 어긋나면 찍은 줄에 그 바늘이 없다).
/// ⚠이 시험은 **일부러 패닉을 일으켜 잡는다** — 로그에 `panicked at` 두 줄이 보이는 것이 정상이고,
///   시험 자체는 초록이다. 패닉 훅은 건드리지 않는다(전역 훅을 바꾸면 **같은 프로세스에서 병렬로 도는
///   남의 시험**의 진단이 사라진다 — 진단을 위해 만든 핀이 진단을 지우는 것은 자기모순이다).
#[test]
fn cite_rejects_a_line_whose_text_does_not_contain_the_needle() {
    let t = "head\nneedle\ntail\n";
    let at = t.find("needle").expect("픽스처에 needle 이 있다");
    assert_eq!(
        cite(t, at, "needle", "synthetic"),
        2,
        "[판별] 바늘이 그 줄에 있으면 cite 는 1-기반 행 번호(2)를 돌려줘야 한다"
    );
    let absent = std::panic::catch_unwind(|| cite(t, at, "absent", "synthetic"));
    assert!(
        absent.is_err(),
        "[판별] cite 가 **그 줄에 없는 바늘**을 거부하지 않았다 — 이 거부가 N4 수리의 전부다. \
         거부가 없으면 행 계수가 어긋났을 때 조용히 틀린 번호를 찍는다"
    );
    let neighbour = std::panic::catch_unwind(|| cite(t, at, "head", "synthetic"));
    assert!(
        neighbour.is_err(),
        "[판별] cite 가 **이웃 줄의 바늘**을 거부하지 않았다 — 행 계수가 1 어긋난 바로 그 고장을 \
         이 단언이 잡는다(codex R1 N4 가 실증한 `main.rs:5037` vs `:5038` 의 모양이다)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 다리 2 — Tauri → 데몬 RPC (변환 없음 · 바이트로 같아야 한다)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn leg2_tauri_rpc_key_matches_the_daemon_reader_key() {
    let shim = read(SHIM);
    let daemon = read(DAEMON);
    let (_camel, arg, _l) = tauri_command_first_arg(&shim, "trash_preview_poll");
    let body = fn_body(&shim, "trash_preview_poll", SHIM);
    let pairs = json_macro_pairs(body, &format!("{SHIM} trash_preview_poll 의 RPC params"));
    let sent = pairs
        .iter()
        .find(|(_, v)| *v == arg)
        .map(|(k, _)| k.clone())
        .unwrap_or_else(|| {
            panic!(
                "[전제] {SHIM} trash_preview_poll 의 `json!` 에 인자 {arg:?} 를 싣는 쌍이 없다 \
                 (꺼낸 쌍: {pairs:?}) — 보내는 키를 가리킬 수 없으면 대조도 할 수 없다"
            )
        });
    let dbody = fn_body(&daemon, "preview_poll", DAEMON);
    let g = dbody
        .find("params.get(\"")
        .unwrap_or_else(|| panic!("[전제] {DAEMON} preview_poll 에 `params.get(\"…\")` 가 없다"));
    let rest = &dbody[g + "params.get(\"".len()..];
    let read_key = rest[..rest.find('"').unwrap_or(0)].to_string();
    assert!(!read_key.is_empty(), "[전제] {DAEMON} preview_poll 의 읽기 키를 꺼내지 못했다");
    assert_eq!(
        sent, read_key,
        "[판별] 잡 식별자 이름이 다리 2 에서 어긋났다 — 이 다리에는 **변환 계층이 없다**(바이트로 같아야 한다).\n\
         · Tauri 가 보내는 RPC params 키({SHIM} trash_preview_poll): {sent:?}\n\
         · 데몬이 읽는 키({DAEMON} preview_poll `params.get`): {read_key:?}\n\
         어긋나면 데몬은 빈 문자열을 잡 id 로 보고 `job_unknown` 을 돌려준다 — 화면은 \
         「만료됐거나 데몬이 다시 떴다」로 읽어 **원인과 다른 사유**를 사람에게 보여준다."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 다리 3 — 데몬 응답 → 화면 (변환 없음 · 다리 1 과 격식이 다른 것이 **정상**이다)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn leg3_daemon_response_key_matches_the_screen_reader_field() {
    let daemon = read(DAEMON);
    let ui = read(UI);
    // 데몬이 답하는 키: preview_start 의 성공 응답에서 `job_id` 변수를 싣는 쌍.
    let sbody = fn_body(&daemon, "preview_start", DAEMON);
    let ok_at = sbody
        .rfind("ok_response(")
        .unwrap_or_else(|| panic!("[전제] {DAEMON} preview_start 에 `ok_response(` 가 없다"));
    let pairs = json_macro_pairs(&sbody[ok_at..], &format!("{DAEMON} preview_start 의 성공 응답"));
    assert_eq!(
        pairs.len(),
        1,
        "[전제] {DAEMON} preview_start 의 성공 응답 쌍이 {}개다 — 이 핀은 **한 칸**(잡 id)을 계약으로 삼는다. \
         칸을 늘렸다면 이 핀도 함께 늘려라",
        pairs.len()
    );
    let (resp_key, _v) = pairs[0].clone();
    // 화면이 읽는 칸: `invoke("trash_preview_start", …) as { <키>?: string }` 의 키.
    let needle = "invoke(\"trash_preview_start\"";
    let at = ui
        .find(needle)
        .unwrap_or_else(|| panic!("[전제] {UI} 에 `{needle}` 호출이 없다"));
    let as_at = ui[at..]
        .find(" as {")
        .unwrap_or_else(|| panic!("[전제] {UI}: trash_preview_start 응답에 타입 주석(` as {{`)이 없다"))
        + at;
    let (open, close) = brace_span(&ui, as_at, &format!("{UI} trash_preview_start 응답 타입"));
    let ty = &ui[open + 1..close];
    let ty_key = ty
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .trim_end_matches('?')
        .trim()
        .to_string();
    assert!(
        is_js_ident(&ty_key),
        "[전제] {UI}:{} 의 응답 타입에서 칸 이름을 꺼내지 못했다(꺼낸 것: {ty_key:?})",
        line_of(&ui, open)
    );
    // 화면이 **실제로 읽는** 자리 — 타입만 맞고 읽기가 어긋나면 컴파일은 통과하고 런타임에
    // undefined 가 흐른다. ★식별자 **경계까지** 읽어 전수로 모은다(N2 수리 · `contains` 아님).
    // ★**종류별로** 하나인지 먼저 본다(N3 수리 · codex R1 Q4). 종전 판은 두 집단을 **합쳐**
    //   `readers.len() == 2` 만 봤다 — 그래서 가드가 **둘**이고 인식된 폴링 전달이 **0** 인 꼴
    //   (`poll(job["job_ix"])` — 대괄호 독자는 이 핀의 모형 밖이다)이 조용히 통과했다.
    //   합계는 **한 집단이 다른 집단의 관측 의무를 대신하게** 만든다. 이제 집단마다 1 을 요구한다.
    let guards = member_reads(&ui, "job?.", UI);
    let polls = member_reads(&ui, "poll(job.", UI);
    assert_eq!(
        guards.len(),
        1,
        "[전제] {UI}: 옵셔널 가드 `job?.<칸>` 자리가 {}곳이다 — 이 핀은 **하나**를 계약으로 삼는다. \
         자리를 늘렸거나 줄였다면 이 핀도 함께 고쳐라(꺼낸 것: {guards:?}). \
         ⛔다른 종류(폴링 전달)의 개수로 이 의무를 대신할 수 없다",
        guards.len()
    );
    assert_eq!(
        polls.len(),
        1,
        "[전제] {UI}: 폴링 전달 `poll(job.<칸>)` 자리가 {}곳이다 — 이 핀은 **하나**를 계약으로 삼는다. \
         0 이면 전달이 사라졌거나 이 핀이 모형화하지 않는 꼴(`job[\"…\"]`·구조분해 등)로 바뀐 것이다 \
         (꺼낸 것: {polls:?}). ⛔가드 개수로 이 의무를 대신할 수 없다",
        polls.len()
    );
    let readers: Vec<(String, usize, &str)> = guards
        .into_iter()
        .map(|(k, at)| (k, at, "옵셔널 가드 `job?.<칸>`"))
        .chain(
            polls
                .into_iter()
                .map(|(k, at)| (k, at, "폴링 전달 `poll(job.<칸>)`")),
        )
        .collect();
    for (name, at, what) in &readers {
        assert_eq!(
            name,
            &resp_key,
            "[판별] 잡 식별자 이름이 다리 3 의 **읽는 자리**에서 어긋났다 — {what}.\n\
             · 데몬이 답하는 키({DAEMON} preview_start): {resp_key:?}\n\
             · 화면이 읽는 이름({UI}:{}): {name:?}\n\
             ★이 자리는 타입 주석과 **따로** 어긋날 수 있다(타입은 맞고 읽기만 틀리면 TS 컴파일은 \
             통과하고 런타임에 `undefined` 가 흘러 「데몬이 job_id 를 주지 않았습니다」로 떨어진다). \
             ⛔부분문자열 비교로는 이 1글자 차이를 못 잡는다 — 그래서 식별자 경계까지 읽어 비교한다.",
            cite(&ui, *at, name, UI)
        );
    }
    assert_eq!(
        resp_key, ty_key,
        "[판별] 잡 식별자 이름이 다리 3 에서 어긋났다 — 응답 경로에는 **변환 계층이 없다**\n\
         (Tauri 명령은 `resp[\"result\"]` 를 그대로 흘린다 · 들어가는 인자만 camel 로 갈아입는다).\n\
         · 데몬이 답하는 키({DAEMON} preview_start): {resp_key:?}\n\
         · 화면이 읽는 칸({UI}:{}): {ty_key:?}\n\
         ★다리 1 의 키가 camel 인 것과 이 자리가 snake 인 것은 **의도된 비대칭**이다 — \
         「일관성」을 이유로 한쪽을 고치면 미리보기가 `job_id` 를 못 읽어 \
         「데몬이 job_id 를 주지 않았습니다」로 떨어진다(데몬은 정상인데 화면이 사람을 오도한다).",
        line_of(&ui, open)
    );
}
