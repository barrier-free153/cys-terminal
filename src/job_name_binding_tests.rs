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

fn line_of(src: &str, byte: usize) -> usize {
    src[..byte].lines().count()
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

/// 화면의 `invoke("<cmd>", { … })` 에서 **첫 키**를 꺼낸다.
/// ★1글자 어긋난 이름도 꺼내야 한다 — 꺼내지 못하면 판별이 아니라 전제에서 죽고, 그 적색은
/// 판별력의 증명이 못 된다. 그래서 식별자 모양이면 무엇이든 꺼낸다(이름을 안다고 가정하지 않는다).
fn ui_invoke_first_key(src: &str, cmd: &str) -> (String, usize) {
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
    let first = inside
        .split(',')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    assert!(
        is_js_ident(&first),
        "[전제] {UI}:{} 의 invoke 인자 객체에서 키를 식별자로 꺼내지 못했다(꺼낸 것: {first:?})",
        line_of(src, open)
    );
    (first, line_of(src, open))
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
    let camel = !attr.contains("rename_all = \"snake_case\"");
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
    (camel, first, line_of(src, at))
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
    let (ui_key, ui_line) = ui_invoke_first_key(&ui, "trash_preview_poll");
    let (camel, arg, shim_line) = tauri_command_first_arg(&shim, "trash_preview_poll");
    let expect = if camel { to_lower_camel(&arg) } else { arg.clone() };
    assert_eq!(
        ui_key, expect,
        "[판별] 잡 식별자 이름이 다리 1 에서 어긋났다.\n\
         · 화면({UI}:{ui_line}) 이 싣는 키: {ui_key:?}\n\
         · Tauri 명령({SHIM}:{shim_line}) 의 인자: {arg:?} · 인자 격식: {}\n\
         · 그 둘이 맞으려면 화면 키는 {expect:?} 여야 한다.\n\
         고치는 자리는 **하나**다: 화면 키를 {expect:?} 로 되돌리거나, Rust 인자 이름을 바꿨다면 \
         화면 키도 그 camel 변환으로 함께 바꿔라. 어긋나면 Tauri 가 인자를 찾지 못해 미리보기 폴링이 \
         첫 호출에서 거부로 떨어진다(사용자가 보는 것: 확인 창이 즉시 사유 없이 닫히거나 스피너가 멈춘다).",
        if camel { "camelCase(기본값)" } else { "snake_case(rename_all 로 뒤집힘)" }
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
    // 화면이 **실제로 읽는** 자리(타입만 맞고 읽기가 어긋나면 런타임에 undefined 다).
    let guard = format!("job?.{ty_key}");
    let pass = format!("poll(job.{ty_key})");
    assert!(
        ui.contains(guard.as_str()) && ui.contains(pass.as_str()),
        "[전제] {UI}: 응답 타입은 {ty_key:?} 인데 실제로 읽는 자리(`{guard}` · `{pass}`)가 없다 — \
         타입 주석만 맞고 읽기가 어긋나면 컴파일은 통과하고 런타임에 undefined 가 흐른다"
    );
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
