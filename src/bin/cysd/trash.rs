//! S8 — 오너가 명령할 때만 하는 휴지통 삭제(`trash.*` RPC · 번들 S8).
//!
//! ## 이 모듈이 **거부**하는 것 — 능력이 아니라 거부가 출하 문구다(ADDENDUM-2 C-R3)
//! · **하위 장치(마운트)를 인증할 수 없으면 그 항목 전체를 거부한다**(REFUSES when it cannot certify a
//!   descendant mount). "하위 마운트를 올바로 처리한다"가 아니다 — 후자는 DESIGN-S8.md:59 를 막은 문장
//!   형상이고, 플랫폼 실행은 여전히 **미측정**이다(기계 1대·OS 1종·한 순간).
//! · 시한이 지난 확인(confirm), 최근(recent) 항목, 최근 여부를 판정할 수 없는 항목도 **거부**다
//!   (ADDENDUM-1 — 판정 불가 = 삭제 안 함).
//!
//! ## 왜 인증기와 제거기가 **같은 집합**인가(ADDENDUM-2 C-R1)
//! 인증기는 항목 뿌리 아래 **모든 노드를 한 번의 순회로** 본다. 순회가 들어가지 못하는 노드(다른 장치·
//! 판독 불가·한도 초과)가 **하나라도** 있으면 그 항목은 통째로 거부된다. 그러므로 인증이 성공했다는 것은
//! "이 뿌리 아래에 장치 경계가 없다"는 뜻이고, 그때 `remove_dir_all` 의 순회는 인증기가 본 집합을
//! **넓힐 수 없다**(심링크는 따라가지 않고 링크 자체를 지운다). 인증을 한 순회로 하고 제거를 다른 순회로
//! 하면서 "오늘은 일치한다"고 말하는 구현은 이 조항을 만족하지 못한다 — 그래서 거부 판정이 제거의
//! **선행조건**이고, 그 판정 하나가 두 집합을 묶는다.
//!
//! ## 이 모듈이 **하지 않는** 것
//! 자동 재개 없음(봉인된 항목은 다음 미리보기 없이 다시 손대지 않는다) · 전체선택 없음 · 클릭만으로 넘어가는
//! 확인 단계 없음 · feed 항목 아님(S7 의 자동 판정자가 닿는 면이 아니다 · D-5/D-7) · 새 CLI 하위명령 없음
//! (`cys` 에는 이 동사가 없다 — 에이전트가 평시 동작으로 닿을 경로를 만들지 않는다).
//!
//! ## 정직한 한계(거버넌스 구분이지 보안 경계가 아니다)
//! 오너 등급은 `operator_token` + pane 무귀속이다. 같은 UID 로 토큰 파일을 읽을 수 있는 프로세스는 raw RPC
//! 로 이 등급을 참칭할 수 있다(`handlers.rs` `caller_is_owner` 문서가 스스로 명시). 이 등급이 닫는 것은
//! **지침을 따르는 정상 노드가 평시 동작으로 오너 입력을 사칭하는 경로**다 — 공용 `cys` CLI 는 어떤 경로에서도
//! 두 토큰 키를 붙이지 않는다. 위조된 삭제는 **최종적**이고 복구 단계는 없다(오너 결정 O0).

use crate::handlers::Reply;
use crate::state::{now_epoch, state_dir, Daemon};
use cys::{err_response, ok_response};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

// ─── ADDENDUM-1 의 두 창(이름 있는 상수 · 이 모듈이 유일한 정의처) ────────────────────────────
/// 최근(recent) 창 — 이보다 **어리면** 쓰는 중일 수 있다고 보고 거부한다.
/// ★`recent` 는 **긍정으로** 판정한다: 나이가 유한하고 이 값 **이상**일 때만 '최근 아님'이다.
/// 더 어림·미래 mtime(음수 나이)·비유한·판독 불가는 전부 `recent` = 거부(fail-closed).
pub const TRASH_RECENT_WINDOW_SECS: u64 = 600;
/// 미리보기 확인(confirm) 레코드의 수명 — **단조 시계**로 잰다(벽시계 점프로 늘어나지 않는다).
/// 같음(=TTL 정확히 도달)은 **만료**다.
pub const TRASH_CONFIRM_TTL_SECS: u64 = 300;
/// 인증기 노드 상한(부서장 D2 채택값). 초과 = 인증 불가 = 그 항목 거부.
pub const TRASH_CERTIFY_MAX_NODES: usize = 5_000_000;
/// 인증기 시간 상한(초 · 부서장 D2 채택값). 초과 = 인증 불가 = 그 항목 거부.
pub const TRASH_CERTIFY_MAX_SECS: u64 = 60;
/// 제거가 길어질 때 **보고**만 하는 임계(초). 중단이 아니다(부서장 D2: report, never abort).
pub const TRASH_REMOVE_STALL_REPORT_SECS: u64 = 120;

/// 봉인 접두 — 삭제 명령을 받은 항목은 먼저 이 이름으로 **봉인**된다(옛 이름으로 계속 쓰는 생산자가
/// 봉인된 객체에 자동으로 닿지 못하게). 봉인 뒤에는 되돌리지 않는다(실패하면 봉인 상태로 남고
/// 자동 재개는 없다).
pub const TRASH_SEAL_PREFIX: &str = ".deleting-";
/// 오래됨 표시 파일 이름(D-1) — reap 은 더 이상 지우지 않고 이 표식만 남긴다.
pub const TRASH_OLD_MARKER: &str = ".trash-old";
/// 삭제 원장(D-4) — 의도·거부·결과가 전부 여기로 간다.
pub const DELETE_LEDGER: &str = "DELETE-LEDGER.jsonl";

// ─── 거부 사유 코드(원장·화면이 같은 문자열을 쓴다) ──────────────────────────────────────────
pub const R_DESC_DEVICE: &str = "uncertifiable-descendant-device";
pub const R_UNREADABLE: &str = "uncertifiable-unreadable";
pub const R_TOO_MANY: &str = "uncertifiable-too-many-nodes";
pub const R_TOO_SLOW: &str = "uncertifiable-too-slow";
pub const R_RECENT: &str = "recent_may_be_in_use";
pub const R_CONFIRM_EXPIRED: &str = "confirm_expired";
pub const R_NOT_IN_PREVIEW: &str = "names_not_in_preview";
pub const R_CHANGED: &str = "entry_changed_since_preview";
pub const R_LEDGER_UNWRITABLE: &str = "refusal_record_unwritable";
pub const R_SEAL_FAILED: &str = "seal_failed";
pub const R_REMOVE_ERROR: &str = "removal_error";
pub const R_NOT_OWNER: &str = "owner_gui_required";
pub const R_NOT_BASE: &str = "base_daemon_only";
pub const R_PRODUCER_ACTIVE: &str = "producer_active";

/// 휴지통 뿌리 — 생산자 셋(cys-dept:94 · javis_org.py:564 · factory_reset.rs:299)과 같은 규약 경로.
pub fn trash_root() -> PathBuf {
    cys::home_dir().join(".local/state/cys-trash")
}

// ─── 한 항목의 인증서 ────────────────────────────────────────────────────────────────────────
/// 인증 성공 — 뿌리 아래 **모든** 노드를 한 순회로 봤고 장치 경계가 없다.
#[derive(Debug, Clone, PartialEq)]
pub struct Certificate {
    /// CD(e) 의 신원 절반 — 항목 뿌리의 (dev, ino).
    pub dev: u64,
    pub ino: u64,
    pub nodes: usize,
    pub bytes: u64,
    pub max_depth: usize,
    /// 가장 **새로운** 노드의 mtime(epoch) — 최근 판정은 뿌리 mtime 이 아니라 이 값으로 한다
    /// (뿌리 mtime 은 깊은 곳의 쓰기를 반영하지 않는다).
    pub newest_mtime: f64,
    /// 봉인 전후 대조용 요약 — (상대경로, 크기, mtime 비트, 장치, inode) 전수의 sha256.
    pub digest: String,
    pub walk_secs: f64,
}

/// 인증 실패 — **항목 전체 거부**다(부분 제거·경고 후 제거·경계까지 제거 전부 금지).
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub code: &'static str,
    /// 인증을 무너뜨린 경계 경로(있으면) — 거부가 읽히게 하는 핵심 정보(C-R2).
    pub boundary: Option<String>,
    pub detail: String,
}

pub type CertifyResult = Result<Certificate, Refusal>;

/// 장치 번호를 어디서 읽는가 — **시험용 주입점**(C-R4 known-failure 픽스처).
/// 이 기계에는 휴지통 뿌리 아래 교차장치 디렉터리가 **0개**라(부서장 실측 · 내 재측정도 0) 실물로는
/// 이 경로를 밟을 수 없다. 그래서 장치 읽기를 seam 으로 두고, 시험이 경계에서 다른 값을 주입한다.
pub type DevOf<'a> = &'a dyn Fn(&Path, &std::fs::Metadata) -> u64;

/// 운영 기본값 — 실제 `st_dev`.
#[cfg(unix)]
pub fn real_dev(_p: &Path, md: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt as _;
    md.dev()
}
/// 윈도우에는 `st_dev` 가 없다 — ★윈도우에서 다르게 도는 자리 1.
/// `volume_serial_number` 는 불안정(nightly)이라 쓰지 않는다. 그래서 윈도우에서는 장치 경계를
/// **증명할 수 없고**, 이 모듈은 증명할 수 없는 것을 성공으로 읽지 않는다 — 윈도우 삭제는
/// 기본 꺼짐이고(feature 로만 켠다) 12.W 가 출하 게이트다.
#[cfg(windows)]
pub fn real_dev(_p: &Path, _md: &std::fs::Metadata) -> u64 {
    0
}

/// 한 항목을 인증한다 — **한 번의 순회**(lstat 전용 · 심링크 미추적).
///
/// 거부 조건(하나라도 걸리면 항목 전체 거부): ①어떤 노드든 장치가 뿌리와 다르다 ②어떤 노드든 lstat·
/// 디렉터리 열기가 실패한다 ③노드 수 상한 초과 ④시간 상한 초과. ★`clock`·`dev_of` 는 주입점이다.
pub fn certify_entry(root: &Path, dev_of: DevOf) -> CertifyResult {
    let t0 = Instant::now();
    let root_md = std::fs::symlink_metadata(root).map_err(|e| Refusal {
        code: R_UNREADABLE,
        boundary: Some(root.display().to_string()),
        detail: format!("뿌리 lstat 실패: {e}"),
    })?;
    if !root_md.is_dir() {
        return Err(Refusal {
            code: R_UNREADABLE,
            boundary: Some(root.display().to_string()),
            detail: "항목 뿌리가 디렉터리가 아니다".into(),
        });
    }
    let root_dev = dev_of(root, &root_md);
    let mut nodes = 0usize;
    let mut bytes = 0u64;
    let mut max_depth = 0usize;
    let mut newest = mtime_secs(&root_md);
    // ★R2 D-r2-3: set_digest 는 **순서 무관 128비트 랩어라운드 합**이다(노드 레코드의 sha256 앞 16바이트).
    //   순서에 의존하지 않으므로 순회 순서가 바뀌어도 같은 집합이면 같은 값이다.
    let mut set_digest: u128 = 0;
    // 순회 순서는 결정론으로 두되(진단 가능), 요약은 순서에 의존하지 않는다.
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        if t0.elapsed().as_secs() > TRASH_CERTIFY_MAX_SECS {
            return Err(Refusal {
                code: R_TOO_SLOW,
                boundary: Some(dir.display().to_string()),
                detail: format!("인증 시간 상한 {TRASH_CERTIFY_MAX_SECS}s 초과 — 이 항목은 삭제 대상이 아니다"),
            });
        }
        let rd = std::fs::read_dir(&dir).map_err(|e| Refusal {
            code: R_UNREADABLE,
            boundary: Some(dir.display().to_string()),
            detail: format!("디렉터리 열기 실패: {e}"),
        })?;
        let mut kids: Vec<PathBuf> = Vec::new();
        for ent in rd {
            let ent = ent.map_err(|e| Refusal {
                code: R_UNREADABLE,
                boundary: Some(dir.display().to_string()),
                detail: format!("디렉터리 항목 읽기 실패: {e}"),
            })?;
            kids.push(ent.path());
        }
        kids.sort();
        for p in kids {
            nodes += 1;
            if nodes > TRASH_CERTIFY_MAX_NODES {
                return Err(Refusal {
                    code: R_TOO_MANY,
                    boundary: Some(p.display().to_string()),
                    detail: format!("노드 상한 {TRASH_CERTIFY_MAX_NODES} 초과 — 이 항목은 삭제 대상이 아니다"),
                });
            }
            let md = std::fs::symlink_metadata(&p).map_err(|e| Refusal {
                code: R_UNREADABLE,
                boundary: Some(p.display().to_string()),
                detail: format!("노드 lstat 실패: {e}"),
            })?;
            let dev = dev_of(&p, &md);
            if dev != root_dev {
                // ★C-R1: 하위 장치를 하나라도 만나면 **항목 전체 거부**. 경계까지 지우지 않는다.
                return Err(Refusal {
                    code: R_DESC_DEVICE,
                    boundary: Some(p.display().to_string()),
                    detail: format!(
                        "하위 노드의 장치({dev})가 항목 뿌리의 장치({root_dev})와 다르다 — \
                         인증할 수 없는 장치 경계이므로 이 항목은 **통째로 거부**된다"
                    ),
                });
            }
            bytes = bytes.saturating_add(md.len());
            let mt = mtime_secs(&md);
            if mt.is_finite() && (!newest.is_finite() || mt > newest) {
                newest = mt;
            }
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().to_string();
            let (_d, ino) = dev_ino(&md);
            let kind = if md.is_dir() { "d" } else if md.file_type().is_symlink() { "l" } else { "f" };
            let mut nh = Sha256::new();
            nh.update(rel.as_bytes());
            nh.update(b"\0");
            nh.update(format!("{kind}:{}:{}:{ino}", md.len(), mtime_bits(&md)).as_bytes());
            let dgt = nh.finalize();
            let mut first16 = [0u8; 16];
            first16.copy_from_slice(&dgt[..16]);
            set_digest = set_digest.wrapping_add(u128::from_be_bytes(first16));
            max_depth = max_depth.max(depth + 1);
            if md.is_dir() {
                stack.push((p, depth + 1));
            }
        }
    }
    let (rdev, rino) = dev_ino(&root_md);
    Ok(Certificate {
        dev: rdev,
        ino: rino,
        nodes,
        bytes,
        max_depth,
        newest_mtime: newest,
        digest: format!("{set_digest:032x}"),
        walk_secs: t0.elapsed().as_secs_f64(),
    })
}

fn mtime_secs(md: &std::fs::Metadata) -> f64 {
    match md.modified() {
        Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs_f64(),
            // UNIX_EPOCH 이전 = 음수. 부호를 살려 둔다(미래·과거 판정은 호출자가 한다).
            Err(e) => -(e.duration().as_secs_f64()),
        },
        Err(_) => f64::NAN,
    }
}
fn mtime_bits(md: &std::fs::Metadata) -> i64 {
    let s = mtime_secs(md);
    if s.is_finite() {
        (s * 1000.0) as i64
    } else {
        i64::MIN
    }
}

/// ★ADDENDUM-1 — '최근'은 **긍정 판정**이다. 나이가 유한하고 창 **이상**일 때만 '최근 아님'.
/// 미래 mtime(음수 나이)·비유한·판독 불가(NaN)는 전부 최근 = 거부.
pub fn is_recent(newest_mtime: f64, now: f64, window_secs: u64) -> bool {
    if !newest_mtime.is_finite() || !now.is_finite() {
        return true;
    }
    let age = now - newest_mtime;
    if !age.is_finite() {
        return true;
    }
    !(age >= window_secs as f64)
}

fn hex32(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

// ─── 삭제 원장(D-4) — 의도·거부·결과·재고가 전부 한 파일로 간다 ──────────────────────────────
/// 원장 한 줄을 append + fsync 한다. **실패를 삼키지 않는다** — 호출자는 실패를 거부로 바꾼다
/// (C-R2: 쓸 수 없는 거부는 그 자체로 진행 거부다).
pub fn ledger_append(dir: &Path, rec: &Value) -> std::io::Result<()> {
    let line = serde_json::to_string(rec)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::create_dir_all(dir)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(DELETE_LEDGER))?;
    f.write_all(format!("{line}\n").as_bytes())?;
    f.sync_all()?;
    Ok(())
}

/// 거부 기록(C-R2) — 거부·그 항목·사유 코드·인증을 무너뜨린 경계 경로를 원장에 남긴다.
/// ★원장에 쓸 수 없으면 `Err` 를 돌려주고, 호출자는 **아무것도 건드리지 않은 채** 거부를 보고한다.
pub fn record_refusal(
    dir: &Path,
    order_id: &str,
    name: &str,
    r: &Refusal,
) -> std::io::Result<()> {
    ledger_append(
        dir,
        &json!({
            "kind": "refusal",
            "ts": now_epoch(),
            "order_id": order_id,
            "entry": name,
            "reason_code": r.code,
            "boundary": r.boundary,
            "detail": r.detail,
            // D-e: origin:"owner_gui" 는 드롭됐다 — 토큰 보유자라는 사실만 적는다(사람 증명 아님).
            "identity": "token_holder",
            "human_verified": false,
        }),
    )
}

// ─── 확인(confirm) 레코드 — 미리보기가 발급하고 명령이 한 번 쓴다 ────────────────────────────
#[derive(Debug, Clone)]
pub struct Confirm {
    pub id: String,
    pub names: Vec<String>,
    pub digest: String,
    /// 단조 시계 기준 발급 시점(벽시계 점프로 수명이 늘어나지 않는다).
    pub issued: Instant,
    /// 미리보기 때의 항목별 인증서(봉인 전후 대조의 기준값).
    pub certs: HashMap<String, Certificate>,
}

fn confirms() -> &'static Mutex<HashMap<String, Confirm>> {
    static C: OnceLock<Mutex<HashMap<String, Confirm>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// ★같음은 만료다 — 나이가 TTL **이상**이면 쓸 수 없다.
pub fn confirm_expired(age_secs: u64, ttl_secs: u64) -> bool {
    age_secs >= ttl_secs
}

/// 여러 이름의 인증서를 하나의 명령 요약으로 접는다(이름 순 — 결정론).
pub fn order_digest(certs: &HashMap<String, Certificate>) -> String {
    let mut names: Vec<&String> = certs.keys().collect();
    names.sort();
    let mut h = Sha256::new();
    for n in names {
        h.update(n.as_bytes());
        h.update(b"\0");
        h.update(certs[n].digest.as_bytes());
        h.update(b"\n");
    }
    hex32(h.finalize().as_slice())
}

// ─── 명령(order) 상태 — `trash.delete` 는 즉시 돌아오고 `trash.order` 가 상태를 돌려준다 ──────
#[derive(Debug, Clone, PartialEq)]
pub struct EntryOutcome {
    pub name: String,
    pub outcome: String,
    pub reason_code: Option<String>,
    pub boundary: Option<String>,
    pub bytes_before: Option<u64>,
    pub duration_ms: u64,
    /// m3: 제거가 시작된 뒤의 오류는 **남은 것이 있을 수 있다**고 정직하게 말한다.
    pub remaining: Option<usize>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OrderState {
    pub order_id: String,
    pub started: f64,
    pub finished: Option<f64>,
    pub per_entry: Vec<EntryOutcome>,
    pub running_long: bool,
}

fn orders() -> &'static Mutex<HashMap<String, OrderState>> {
    static O: OnceLock<Mutex<HashMap<String, OrderState>>> = OnceLock::new();
    O.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 단일 삭제자 — **기다리지 않는** 빗장이다(대기는 그 자체가 stall 이다).
/// `MutexGuard` 는 스레드를 건너가지 못하므로(!Send) 원자 플래그를 쓴다 — 잡기와 놓기가
/// 서로 다른 스레드에서 일어나기 때문이다.
static DELETER_BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// 잡혔으면 true. 놓는 것은 `deleter_release`.
fn deleter_try_acquire() -> bool {
    DELETER_BUSY
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        )
        .is_ok()
}
fn deleter_release() {
    DELETER_BUSY.store(false, std::sync::atomic::Ordering::Release);
}

/// 시험 전용 — 모듈 전역 상태를 비운다(검체 간 격리).
#[cfg(test)]
pub fn reset_state_for_test() {
    confirms().lock().unwrap().clear();
    orders().lock().unwrap().clear();
}

/// ★시험 전용 단계 훅(R2 §3.5 의 `after_seal`) — 봉인과 대조 **사이**에 끼어드는 쓰기를 만든다.
/// 이 자리를 시험이 잡을 수 없으면 봉인 뒤 대조 조항은 어떤 시험에도 하중이 걸리지 않는다
/// (실측: 이 훅이 없을 때 변이 M5 가 **생존**했다).
/// ★`cargo test` 는 시험을 **병렬**로 돌린다. 훅이 전역 하나면 남의 시험에 끼어든다(실측: 그래서
/// 대조 실행이 1건 빨갰다). 그래서 **등록한 뿌리 아래에서만** 발화한다 — 시험 격리는 기능의
/// 일부다(오염된 초록은 초록이 아니다).
#[cfg(test)]
pub static AFTER_SEAL_HOOK: Mutex<Option<(PathBuf, fn(&Path))>> = Mutex::new(None);

// ─── 한 항목의 집행(순차) — 티켓 ⒡ 의 순서 그대로 ────────────────────────────────────────────
/// 집행 한 항목: 신원 재확인 → 봉인 rename → 재확인 → `remove_dir_all` → lstat 검증.
/// ★호출 전에 인증(certify)이 성공했어야 한다 — 인증 실패는 여기 오지 않고 거부로 끝난다.
#[allow(clippy::too_many_arguments)]
pub fn execute_one(
    root: &Path,
    name: &str,
    baseline: &Certificate,
    dev_of: DevOf,
    now: f64,
    state: &Path,
    order_id: &str,
) -> EntryOutcome {
    let t0 = Instant::now();
    let src = root.join(name);
    let mut out = EntryOutcome {
        name: name.to_string(),
        outcome: "refused".into(),
        reason_code: None,
        boundary: None,
        bytes_before: Some(baseline.bytes),
        duration_ms: 0,
        remaining: None,
        errors: vec![],
    };
    // 이미 사라졌는가 — 손댈 것이 없다(성공도 실패도 아닌 사실).
    if std::fs::symlink_metadata(&src).is_err() {
        out.outcome = "already_gone".into();
        out.duration_ms = t0.elapsed().as_millis() as u64;
        return out;
    }
    // (2) 신원 재확인 — 미리보기 이후 바뀌었으면 손대지 않는다.
    let fresh = match certify_entry(&src, dev_of) {
        Ok(c) => c,
        Err(r) => {
            out.reason_code = Some(r.code.to_string());
            out.boundary = r.boundary.clone();
            let _ = record_refusal(state, order_id, name, &r);
            out.duration_ms = t0.elapsed().as_millis() as u64;
            return out;
        }
    };
    if fresh.digest != baseline.digest {
        let r = Refusal {
            code: R_CHANGED,
            boundary: None,
            detail: "미리보기 이후 내용이 바뀌었다 — 새 미리보기가 필요하다".into(),
        };
        out.reason_code = Some(r.code.to_string());
        let _ = record_refusal(state, order_id, name, &r);
        out.duration_ms = t0.elapsed().as_millis() as u64;
        return out;
    }
    if is_recent(fresh.newest_mtime, now, TRASH_RECENT_WINDOW_SECS) {
        let r = Refusal {
            code: R_RECENT,
            boundary: None,
            detail: format!(
                "가장 새로운 노드의 나이가 {TRASH_RECENT_WINDOW_SECS}s 미만이거나 판정 불가 — 쓰는 중일 수 있어 거부"
            ),
        };
        out.reason_code = Some(r.code.to_string());
        let _ = record_refusal(state, order_id, name, &r);
        out.duration_ms = t0.elapsed().as_millis() as u64;
        return out;
    }
    // (3) 봉인 — 옛 이름으로 계속 쓰는 생산자가 봉인된 객체에 자동으로 닿지 못하게 한다.
    let sealed = root.join(format!("{TRASH_SEAL_PREFIX}{name}"));
    if sealed.exists() {
        let r = Refusal {
            code: R_SEAL_FAILED,
            boundary: Some(sealed.display().to_string()),
            detail: "같은 이름의 봉인이 이미 있다(중단된 명령) — 자동 재개하지 않는다".into(),
        };
        out.reason_code = Some(r.code.to_string());
        out.boundary = r.boundary.clone();
        let _ = record_refusal(state, order_id, name, &r);
        out.duration_ms = t0.elapsed().as_millis() as u64;
        return out;
    }
    if let Err(e) = std::fs::rename(&src, &sealed) {
        let r = Refusal {
            code: R_SEAL_FAILED,
            boundary: Some(src.display().to_string()),
            detail: format!("봉인 rename 실패: {e}"),
        };
        out.reason_code = Some(r.code.to_string());
        let _ = record_refusal(state, order_id, name, &r);
        out.duration_ms = t0.elapsed().as_millis() as u64;
        return out;
    }
    #[cfg(test)]
    {
        let g = AFTER_SEAL_HOOK.lock().unwrap();
        if let Some((scope, h)) = g.as_ref() {
            if sealed.starts_with(scope) {
                h(&sealed);
            }
        }
    }
    // (4) 봉인 뒤 재확인 — 다르면 **봉인 상태로 남긴다**(되돌리지 않는다 · 자동 재개 없음).
    match certify_entry(&sealed, dev_of) {
        Ok(c) if c.digest == baseline.digest => {}
        Ok(_) => {
            let r = Refusal {
                code: R_CHANGED,
                boundary: Some(sealed.display().to_string()),
                detail: "봉인 뒤 대조가 어긋났다 — 봉인 상태로 남기고 중단(되돌리지 않는다)".into(),
            };
            out.reason_code = Some(r.code.to_string());
            out.boundary = r.boundary.clone();
            out.outcome = "sealed_not_removed".into();
            let _ = record_refusal(state, order_id, name, &r);
            out.duration_ms = t0.elapsed().as_millis() as u64;
            return out;
        }
        Err(r) => {
            out.reason_code = Some(r.code.to_string());
            out.boundary = r.boundary.clone();
            out.outcome = "sealed_not_removed".into();
            let _ = record_refusal(state, order_id, name, &r);
            out.duration_ms = t0.elapsed().as_millis() as u64;
            return out;
        }
    }
    // (5) 제거 — 인증이 "이 뿌리 아래 장치 경계 없음"을 이미 확정했으므로 이 순회는 인증 집합을
    //     넓힐 수 없다(C-R1). 심링크는 따라가지 않고 링크 자체가 지워진다.
    if let Err(e) = std::fs::remove_dir_all(&sealed) {
        // m3: 제거가 **시작된 뒤**의 오류다 — 남은 것이 있을 수 있다고 적는다(무결 주장 금지).
        let remaining = certify_entry(&sealed, dev_of).map(|c| c.nodes).ok();
        // partial = 제거가 시작된 뒤 멈췄다(남은 것이 있을 수 있다 · m3).
        out.outcome = "partial".into();
        out.reason_code = Some(R_REMOVE_ERROR.to_string());
        out.remaining = remaining;
        out.errors.push(format!("{e}"));
        let r = Refusal {
            code: R_REMOVE_ERROR,
            boundary: Some(sealed.display().to_string()),
            detail: format!(
                "제거 중 오류: {e} — 봉인된 잔여가 남아 있을 수 있다(남은 노드 {remaining:?}). \
                 자동 재개하지 않는다; 새 미리보기와 새 명령이 필요하다"
            ),
        };
        let _ = record_refusal(state, order_id, name, &r);
        out.duration_ms = t0.elapsed().as_millis() as u64;
        return out;
    }
    // (6) lstat 검증 — 없어졌는가.
    if std::fs::symlink_metadata(&sealed).is_ok() {
        out.outcome = "removal_error".into();
        out.reason_code = Some(R_REMOVE_ERROR.to_string());
        out.errors.push("제거 뒤에도 봉인 경로가 남아 있다".into());
        out.duration_ms = t0.elapsed().as_millis() as u64;
        return out;
    }
    out.outcome = "deleted_verified".into();
    out.duration_ms = t0.elapsed().as_millis() as u64;
    out
}

// ─── 크기 캐시 — ★`trash.list` 는 **걷지 않는다**(DESIGN-S8 4.1) ─────────────────────────────
/// `size-cache.json` — 이름 + 장치 + inode + mtime 으로 키를 만든다. 키가 바뀌면 캐시는 무효다.
/// 걷기는 `trash.preview` 가 할 때만 일어나고(오너가 그 항목을 고른 순간) 그 결과가 여기 쌓인다.
/// 그래서 3.5 GB 합계 계산이 목록 때문에 일어나지 않는다 — 창이 멈추지 않는 이유는 이 한 줄이다.
pub fn size_cache_path(root: &Path) -> PathBuf {
    root.join("size-cache.json")
}
pub fn size_cache_key(name: &str, dev: u64, ino: u64, mtime_bits: i64) -> String {
    format!("{name}|{dev}|{ino}|{mtime_bits}")
}
fn size_cache_read(root: &Path) -> serde_json::Map<String, Value> {
    std::fs::read_to_string(size_cache_path(root))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}
fn size_cache_write(root: &Path, key: &str, c: &Certificate) {
    let mut m = size_cache_read(root);
    m.insert(
        key.to_string(),
        json!({"bytes": c.bytes, "nodes": c.nodes, "max_depth": c.max_depth,
               "newest_mtime": c.newest_mtime, "measured_at": now_epoch(), "walk_secs": c.walk_secs}),
    );
    // 유계: 최근 64개만 들고 있는다(휴지통 뿌리의 자식 수는 한 자리 수다 — 넉넉한 상한).
    if m.len() > 64 {
        let mut keys: Vec<(String, f64)> = m
            .iter()
            .map(|(k, v)| (k.clone(), v.get("measured_at").and_then(Value::as_f64).unwrap_or(0.0)))
            .collect();
        keys.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (k, _) in keys.into_iter().take(m.len() - 64) {
            m.remove(&k);
        }
    }
    let _ = std::fs::write(size_cache_path(root), Value::Object(m).to_string());
}

/// ★**조언용** 유일본 표지(advisory only · 아무 것도 막지 않는다).
/// 근거(B-064 실측): 부서 격리분의 `account`·`pack-dept-*`·`state`(대화기억)는 그 기계에 **유일본**이고,
/// `factory-reset-*` 는 초기화 당시의 상태 전부다. 표지는 오너가 **보고 판단하도록** 띄우는 것이고,
/// 판정에 쓰이지 않는다 — 조언 신호를 비가역 행동의 하중으로 쓰면 안 된다(부서장 Q1 결정).
pub fn sole_copy_advisory(name: &str, children: &[String]) -> bool {
    if name.starts_with("factory-reset-") {
        return true;
    }
    children.iter().any(|c| {
        c == "state" || c == "account" || c.starts_with("pack-dept-") || c == "pack"
    })
}

/// 이름의 스탬프에서 나이를 읽는다(D-r2-9 — 유예 시계는 **이름의 스탬프**이고 mtime 이 아니다).
/// 생산자가 둘이고 형식이 둘이다: ①epoch 초(cys-dept:271 · javis_org.py:600) ②`%Y%m%dT%H%M%SZ`
/// (factory_reset.rs:546). **둘 다 받고, 알 수 없으면 '오래되지 않음'** 으로 본다(미지 = 안 지운다).
pub fn stamp_epoch_from_name(name: &str) -> Option<f64> {
    let tail = name.rsplit('-').next()?;
    // ② 압축 UTC
    if tail.len() == 16 && tail.ends_with('Z') && tail.as_bytes()[8] == b'T' {
        let (d, t) = tail.split_at(8);
        let (h, mi, s) = (&t[1..3], &t[3..5], &t[5..7]);
        let y: i64 = d[0..4].parse().ok()?;
        let mo: i64 = d[4..6].parse().ok()?;
        let da: i64 = d[6..8].parse().ok()?;
        let hh: i64 = h.parse().ok()?;
        let mm: i64 = mi.parse().ok()?;
        let ss: i64 = s.parse().ok()?;
        // 민간 그레고리력 → epoch (순수 산술 · 윤년 포함 · 외부 의존 없음).
        let days = days_from_civil(y, mo, da)?;
        return Some((days * 86400 + hh * 3600 + mm * 60 + ss) as f64);
    }
    // ① epoch 초
    if tail.len() >= 9 && tail.chars().all(|c| c.is_ascii_digit()) {
        return tail.parse::<f64>().ok();
    }
    None
}
/// Howard Hinnant 의 days_from_civil(순수) — 1970-01-01 기준 일수.
fn days_from_civil(y: i64, m: i64, d: i64) -> Option<i64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// 이름 스탬프 기준 TTL 초과 여부 — **알 수 없으면 false**(오래되지 않음 = 표시도 안 한다).
pub fn is_old_by_stamp(name: &str, now: f64, ttl_days: f64) -> bool {
    match stamp_epoch_from_name(name) {
        Some(st) if st.is_finite() && now.is_finite() => (now - st) > ttl_days * 86400.0,
        _ => false,
    }
}

// ─── RPC ─────────────────────────────────────────────────────────────────────────────────────
/// `trash.*` 의 단일 진입점. `owner_ok` 는 **handlers 가 판정해** 넘긴다
/// (`from_sid.is_none() && operator_token 일치` — `owner_token` 은 읽지 않는다: 더 넓은 부착면이라
/// S6·S7 이 그 키를 넓히면 삭제까지 함께 넓어진다 · DESIGN-S8.md:52).
pub fn handle(
    daemon: &Arc<Daemon>,
    sub: &str,
    params: &Value,
    id: &Value,
    owner_ok: bool,
    caller_pid: Option<u32>,
) -> Reply {
    // 기지 데몬 전용 — 부서 데몬에는 이 동사가 없다(휴지통은 기계에 하나다).
    if cys::is_dept_socket(&daemon.socket_path) {
        return Reply::Single(err_response(
            id,
            R_NOT_BASE,
            "휴지통 삭제는 기지 데몬 전용이다(휴지통은 기계에 하나 · 부서 데몬에는 이 동사가 없다)",
        ));
    }
    let root = trash_root();
    let state = state_dir(&daemon.socket_path);
    // 뿌리가 심링크거나 디렉터리가 아니면 **아무 동사도** 동작하지 않는다(S8:115).
    if root.exists() && !root_sane(&root) {
        return Reply::Single(err_response(
            id,
            "refused",
            "휴지통 뿌리가 디렉터리가 아니거나 심링크다 — 이 상태에서는 동작하지 않는다",
        ));
    }
    match sub {
        // 읽기 전용 · 어떤 로컬 호출자든 — 감사자가 거부당하지 않게(에이전트는 이미 `ls` 할 수 있고
        // 비밀은 돌려주지 않는다). ★걷지 않는다: 깊이 1 과 캐시만 읽는다.
        "list" => Reply::Single(ok_response(id, list_payload(&root, &state))),
        "preview" => {
            if !owner_ok {
                return Reply::Single(err_response(id, R_NOT_OWNER, OWNER_REQUIRED_MSG));
            }
            preview(id, &root, &state, params)
        }
        "delete" => {
            if !owner_ok {
                return Reply::Single(err_response(id, R_NOT_OWNER, OWNER_REQUIRED_MSG));
            }
            delete(id, &root, &state, params, caller_pid)
        }
        "order" => {
            let oid = params.get("order_id").and_then(Value::as_str).unwrap_or("");
            let o = orders().lock().unwrap().get(oid).cloned();
            match o {
                Some(o) => Reply::Single(ok_response(id, order_payload(&o))),
                None => Reply::Single(err_response(
                    id,
                    "not_found",
                    "그 order_id 를 이 데몬이 모른다 — 데몬이 다시 뜨면 원장(DELETE-LEDGER.jsonl)이 정본이다",
                )),
            }
        }
        other => Reply::Single(err_response(
            id,
            "unknown_method",
            &format!("trash.{other} 는 없는 동사다 — list|preview|delete|order"),
        )),
    }
}

pub const OWNER_REQUIRED_MSG: &str = "휴지통 삭제는 앱(오너 GUI)에서만 명령한다 — \
    operator token 과 pane 에 붙지 않은 호출자만 이 동사를 쓴다. \
    ★정직한 한계: 같은 UID 로 토큰 파일을 읽는 프로세스는 이 등급을 참칭할 수 있다(거버넌스 구분이지 보안 경계가 아니다).";

fn order_payload(o: &OrderState) -> Value {
    json!({
        "order_id": o.order_id, "started": o.started, "finished": o.finished,
        "running_long": o.running_long,
        "per_entry": o.per_entry.iter().map(|e| json!({
            "name": e.name, "outcome": e.outcome, "reason_code": e.reason_code,
            "boundary": e.boundary, "bytes_before": e.bytes_before,
            "duration_ms": e.duration_ms, "remaining": e.remaining, "errors": e.errors,
        })).collect::<Vec<_>>(),
    })
}

/// 목록 — 걷지 않는다. 이름·나이·깊이1 내용·유일본 조언 표지·오래됨 표시·캐시된 크기만.
pub fn list_payload(root: &Path, state: &Path) -> Value {
    let now = now_epoch();
    let cache = size_cache_read(root);
    let mut entries = Vec::new();
    let mut strays = Vec::new();
    if let Ok(rd) = std::fs::read_dir(root) {
        let mut names: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        names.sort();
        for p in names {
            let name = match p.file_name().and_then(|s| s.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if name == "size-cache.json" {
                continue;
            }
            let md = match std::fs::symlink_metadata(&p) {
                Ok(m) => m,
                Err(e) => {
                    strays.push(json!({"name": name, "why": format!("lstat 실패: {e}")}));
                    continue;
                }
            };
            if !md.is_dir() {
                strays.push(json!({"name": name, "why": "디렉터리가 아니다"}));
                continue;
            }
            // 봉인 잔재 = 중단된 명령. 자동 재개하지 않는다.
            let interrupted = name.starts_with(TRASH_SEAL_PREFIX);
            let mut children: Vec<String> = Vec::new();
            if let Ok(kids) = std::fs::read_dir(&p) {
                for k in kids.flatten() {
                    if let Some(n) = k.file_name().to_str() {
                        // ★점 파일은 **첫 단계 목록에 넣지 않는다**(설계 r2 §3.4: "`.producer` 는
                        //   점 파일이다 — 첫 단계 내용도 아니고 kind 판정에도 쓰이지 않지만 내용
                        //   집합의 노드다"). 내가 f1c4afb6 에서 표식을 쓰기 시작한 뒤로 이 목록에
                        //   `.producer` 가 섞여 오너가 만들지 않은 파일이 보이고 개수가 하나 더
                        //   많았다(성찰 A-2 에서 발견). digest 는 파일시스템을 직접 걷으므로
                        //   이 필터와 무관하다 — **보여주는 목록만** 걸러진다.
                        if n.starts_with('.') {
                            continue;
                        }
                        children.push(n.to_string());
                    }
                }
            }
            children.sort();
            let (dev, ino) = dev_ino(&md);
            let key = size_cache_key(&name, dev, ino, mtime_bits(&md));
            let size = cache.get(&key).cloned();
            let pst = producer_state(&p, &real_alive);
            entries.push(json!({
                "name": name,
                "interrupted_deleting": interrupted,
                "root_mtime_age_secs": (now - mtime_secs(&md)).max(0.0),
                "stamp_epoch": stamp_epoch_from_name(&name),
                "old_by_stamp": is_old_by_stamp(&name, now, ttl_days_default()),
                "old_marked": p.join(TRASH_OLD_MARKER).exists(),
                "children_depth1": children.clone(),
                "children_total": children.len(),
                "sole_copy_advisory": sole_copy_advisory(&name, &children),
                // ★표식 상태는 **한 번만** 읽는다(성찰 A-1). 두 번 읽으면 두 읽기 사이에 생산자가
                //   끝날 수 있어 "생산자가 돌고 있다" 표지와 `deletable: true` 가 같은 줄에 실릴 수
                //   있었다(스스로 모순되는 줄 · 안전하지만 품질 결함). 비용도 절반이다.
                "producer_chip": producer_chip(&pst),
                "deletable": !interrupted && !producer_blocks(&pst),
                // ★크기는 **측정됐을 때만** 숫자다. 아니면 pending — 목록이 걷지 않기 때문이다.
                "size": match size { Some(s) => json!({"state": "measured", "value": s}),
                                     None => json!({"state": "pending"}) },
            }));
        }
    }
    let ledger_ok = ledger_writable(state);
    json!({
        "root": root.display().to_string(),
        "entries": entries,
        "strays": strays,
        "ledger_writable": ledger_ok,
        "ledger_state": if ledger_ok { Value::Null } else { json!(R_LEDGER_UNWRITABLE) },
        "constants": {
            "recent_window_secs": TRASH_RECENT_WINDOW_SECS,
            "confirm_ttl_secs": TRASH_CONFIRM_TTL_SECS,
            "certify_max_nodes": TRASH_CERTIFY_MAX_NODES,
            "certify_max_secs": TRASH_CERTIFY_MAX_SECS,
            "remove_stall_report_secs": TRASH_REMOVE_STALL_REPORT_SECS,
        },
        "delete_supported": delete_supported(),
    })
}

fn ttl_days_default() -> f64 {
    std::env::var("CYS_TRASH_TTL_DAYS")
        .ok()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(14.0)
}

#[cfg(unix)]
fn dev_ino(md: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    (md.dev(), md.ino())
}
#[cfg(windows)]
fn dev_ino(_md: &std::fs::Metadata) -> (u64, u64) {
    // ★윈도우에서 다르게 도는 자리 2 — 안정 API 로 dev·ino 를 얻을 수 없다. 캐시 키는
    // 이름+mtime 만으로 약해지고, 그 약함이 윈도우 삭제를 기본 꺼짐으로 두는 이유의 일부다.
    (0, 0)
}

/// 윈도우 삭제는 **검증된 빌드에서만** 켜진다(12.W 가 돌지 않으면 꺼진 채로 출하된다).
pub fn delete_supported() -> bool {
    if cfg!(windows) {
        cfg!(feature = "trash_win_validated")
    } else {
        true
    }
}

fn ledger_writable(state: &Path) -> bool {
    // 쓸 수 있는가를 **쓰지 않고** 본다: 디렉터리 존재 + append 열기만 시도하고 닫는다.
    if std::fs::create_dir_all(state).is_err() {
        return false;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(state.join(DELETE_LEDGER))
        .is_ok()
}

/// 미리보기 — 오너가 고른 이름들을 인증하고, 확인(confirm) 레코드를 발급한다.
/// ★거부는 여기서 끝난다: 인증할 수 없는 항목은 **확인에 담기지 않으므로** 명령의 대상이 될 수 없다.
fn preview(id: &Value, root: &Path, state: &Path, params: &Value) -> Reply {
    let names = match str_list(params, "names") {
        Some(v) if !v.is_empty() => v,
        _ => {
            return Reply::Single(err_response(
                id,
                "invalid_params",
                "names 가 없다 — 미리보기는 이름을 하나씩 받는다(전체선택은 없다)",
            ))
        }
    };
    let now = now_epoch();
    let mut certs: HashMap<String, Certificate> = HashMap::new();
    let mut shown = Vec::new();
    let mut refusals = Vec::new();
    for name in &names {
        if !safe_name(name) {
            return Reply::Single(err_response(
                id,
                "invalid_params",
                "이름에 경로 구분자·상대 성분이 있다 — 휴지통 뿌리의 직속 자식 이름만 받는다",
            ));
        }
        let p = root.join(name);
        // D-r2-5 M3: 생산자가 돌고 있으면(또는 생존 판정 불가면) 이 항목은 거부한다.
        let pst = producer_state(&p, &real_alive);
        if producer_blocks(&pst) {
            let r = Refusal {
                code: R_PRODUCER_ACTIVE,
                boundary: Some(p.join(".producer").display().to_string()),
                detail: format!("생산자 표식 상태: {} — 거부", producer_chip(&pst)),
            };
            if record_refusal(state, "(preview)", name, &r).is_err() {
                return Reply::Single(err_response(id, R_LEDGER_UNWRITABLE, LEDGER_FAIL_MSG));
            }
            refusals.push(refusal_json(name, &r));
            continue;
        }
        match certify_entry(&p, &real_dev) {
            Ok(c) => {
                let recent = is_recent(c.newest_mtime, now, TRASH_RECENT_WINDOW_SECS);
                if let Ok(md) = std::fs::symlink_metadata(&p) {
                    let (dev, ino) = dev_ino(&md);
                    size_cache_write(root, &size_cache_key(name, dev, ino, mtime_bits(&md)), &c);
                }
                if recent {
                    let r = Refusal {
                        code: R_RECENT,
                        boundary: None,
                        detail: format!(
                            "가장 새로운 노드가 {TRASH_RECENT_WINDOW_SECS}s 안에 쓰였거나 판정 불가 — 쓰는 중일 수 있어 거부"
                        ),
                    };
                    if record_refusal(state, "(preview)", name, &r).is_err() {
                        return Reply::Single(err_response(id, R_LEDGER_UNWRITABLE, LEDGER_FAIL_MSG));
                    }
                    refusals.push(refusal_json(name, &r));
                    continue;
                }
                shown.push(json!({
                    "name": name, "nodes": c.nodes, "bytes": c.bytes, "max_depth": c.max_depth,
                    "newest_mtime": c.newest_mtime, "digest": c.digest, "walk_secs": c.walk_secs,
                }));
                certs.insert(name.clone(), c);
            }
            Err(r) => {
                // ★C-R1 + C-R2: 인증 실패 = 그 항목 **전체** 거부, 그리고 그 거부는 원장에 남는다.
                if record_refusal(state, "(preview)", name, &r).is_err() {
                    return Reply::Single(err_response(id, R_LEDGER_UNWRITABLE, LEDGER_FAIL_MSG));
                }
                refusals.push(refusal_json(name, &r));
            }
        }
    }
    if certs.is_empty() {
        return Reply::Single(ok_response(
            id,
            json!({"confirm_id": Value::Null, "entries": shown, "refused": refusals,
                   "digest": Value::Null, "confirm_ttl_secs": TRASH_CONFIRM_TTL_SECS}),
        ));
    }
    let digest = order_digest(&certs);
    let cid = new_id("cf");
    confirms().lock().unwrap().insert(
        cid.clone(),
        Confirm {
            id: cid.clone(),
            names: {
                let mut k: Vec<String> = certs.keys().cloned().collect();
                k.sort();
                k
            },
            digest: digest.clone(),
            issued: Instant::now(),
            certs,
        },
    );
    Reply::Single(ok_response(
        id,
        json!({"confirm_id": cid, "entries": shown, "refused": refusals, "digest": digest,
               "confirm_ttl_secs": TRASH_CONFIRM_TTL_SECS,
               "delete_supported": delete_supported()}),
    ))
}

pub const LEDGER_FAIL_MSG: &str = "거부를 원장에 기록할 수 없어 진행하지 않았다 — \
    읽을 수 없는 거부는 거부가 아니다(아무 것도 건드리지 않았다)";

fn refusal_json(name: &str, r: &Refusal) -> Value {
    json!({"name": name, "reason_code": r.code, "boundary": r.boundary, "detail": r.detail})
}

/// 명령 — 확인을 한 번 쓰고, 의도를 원장에 적고, 백그라운드에서 순차 집행한다. 즉시 돌아온다.
fn delete(id: &Value, root: &Path, state: &Path, params: &Value, caller_pid: Option<u32>) -> Reply {
    if delete_killswitched() {
        return Reply::Single(err_response(
            id,
            "refused",
            "CYS_TRASH_DELETE=off — 이 기계에서 삭제가 꺼져 있다(목록·표시는 그대로 동작한다)",
        ));
    }
    if !delete_supported() {
        return Reply::Single(err_response(
            id,
            "unsupported_platform_pending_validation",
            "이 빌드에서는 휴지통 삭제가 꺼져 있다 — 윈도우 삭제는 12.W 검증을 통과한 빌드에서만 켠다 \
             (목록·표시는 그대로 동작한다)",
        ));
    }
    let cid = params.get("confirm_id").and_then(Value::as_str).unwrap_or("").to_string();
    let names = str_list(params, "names").unwrap_or_default();
    let digest = params.get("digest").and_then(Value::as_str).unwrap_or("").to_string();
    let c = match confirms().lock().unwrap().remove(&cid) {
        // ★한 번만 쓴다 — remove 로 꺼내므로 재생(replay)이 구조적으로 불가능하다.
        Some(c) => c,
        None => {
            return Reply::Single(err_response(
                id,
                R_CONFIRM_EXPIRED,
                "확인 레코드가 없다(이미 썼거나 데몬이 다시 떴다) — 새 미리보기가 필요하다",
            ))
        }
    };
    debug_assert_eq!(c.id, cid, "확인 레코드의 키와 본문 id 가 어긋났다");
    let age = c.issued.elapsed().as_secs();
    if confirm_expired(age, TRASH_CONFIRM_TTL_SECS) {
        let r = Refusal {
            code: R_CONFIRM_EXPIRED,
            boundary: None,
            detail: format!("확인이 {age}s 전에 발급됐다(TTL {TRASH_CONFIRM_TTL_SECS}s · 같음도 만료) — 새 미리보기가 필요하다"),
        };
        if record_refusal(state, "(order)", &cid, &r).is_err() {
            return Reply::Single(err_response(id, R_LEDGER_UNWRITABLE, LEDGER_FAIL_MSG));
        }
        return Reply::Single(err_response(id, r.code, &r.detail));
    }
    let mut asked = names.clone();
    asked.sort();
    if asked != c.names || digest != c.digest {
        let r = Refusal {
            code: R_NOT_IN_PREVIEW,
            boundary: None,
            detail: "명령의 이름 집합·요약이 미리보기와 다르다 — 새 미리보기가 필요하다".into(),
        };
        if record_refusal(state, "(order)", &cid, &r).is_err() {
            return Reply::Single(err_response(id, R_LEDGER_UNWRITABLE, LEDGER_FAIL_MSG));
        }
        return Reply::Single(err_response(id, r.code, &r.detail));
    }
    // 단일 삭제자 — try-lock. 기다리지 않는다(기다리면 그것이 stall 이다).
    if !deleter_try_acquire() {
        return Reply::Single(err_response(
            id,
            "busy",
            "다른 삭제 명령이 진행 중이다 — 끝난 뒤 다시 명령한다(대기하지 않는다)",
        ));
    }
    let order_id = new_id("od");
    // (1) 의도 기록 — **첫 바이트 전에**. 쓸 수 없으면 아무것도 하지 않는다.
    let intent = json!({
        "kind": "intent", "ts": now_epoch(), "order_id": order_id,
        "command": {"identity": "token_holder",
                    "identity_verified": "operator_token_and_not_in_pane_of_this_daemon",
                    "human_verified": false, "operator_token_ok": true,
                    "caller_pid": caller_pid, "caller_in_pane": false,
                    // 신고값이며 검증되지 않는다 — 읽는 사람이 오해하지 않게 이름에 적는다.
                    "claimed_client_unverified": params.get("client").and_then(Value::as_str),
                    "confirm_id": cid, "digest": c.digest},
        "entries": c.names.iter().map(|n| {
            let cert = &c.certs[n];
            json!({"name": n, "nodes": cert.nodes, "bytes": cert.bytes,
                   "newest_mtime": cert.newest_mtime, "digest": cert.digest})
        }).collect::<Vec<_>>(),
    });
    if let Err(e) = ledger_append(state, &intent) {
        deleter_release();
        return Reply::Single(err_response(
            id,
            R_LEDGER_UNWRITABLE,
            &format!("{LEDGER_FAIL_MSG} (의도 기록 실패: {e})"),
        ));
    }
    orders().lock().unwrap().insert(
        order_id.clone(),
        OrderState {
            order_id: order_id.clone(),
            started: now_epoch(),
            finished: None,
            per_entry: vec![],
            running_long: false,
        },
    );
    // 집행은 별도 스레드 — dispatch 스레드를 쥐지 않는다(큐·배달 경로에 영향 0).
    let root2 = root.to_path_buf();
    let state2 = state.to_path_buf();
    let oid2 = order_id.clone();
    // 빗장은 **이미 잡혀 있다**(위 try_acquire). 집행 스레드가 끝에서 놓는다.
    std::thread::spawn(move || {
        let t0 = Instant::now();
        let mut outs = Vec::new();
        for name in &c.names {
            // CK: 항목 **사이마다** 확인 수명을 다시 본다(지났으면 남은 항목은 손대지 않는다).
            if confirm_expired(c.issued.elapsed().as_secs(), TRASH_CONFIRM_TTL_SECS) {
                let r = Refusal {
                    code: R_CONFIRM_EXPIRED,
                    boundary: None,
                    detail: "집행 중 확인이 만료됐다 — 남은 항목은 손대지 않는다(새 미리보기 필요)".into(),
                };
                let _ = record_refusal(&state2, &oid2, name, &r);
                outs.push(EntryOutcome {
                    name: name.clone(),
                    outcome: "refused".into(),
                    reason_code: Some(r.code.to_string()),
                    boundary: None,
                    bytes_before: None,
                    duration_ms: 0,
                    remaining: None,
                    errors: vec![],
                });
                continue;
            }
            let out = execute_one(
                &root2, name, &c.certs[name], &real_dev, now_epoch(), &state2, &oid2,
            );
            outs.push(out);
            if t0.elapsed().as_secs() >= TRASH_REMOVE_STALL_REPORT_SECS {
                if let Some(o) = orders().lock().unwrap().get_mut(&oid2) {
                    o.running_long = true; // 보고만 — 중단이 아니다.
                }
            }
            if let Some(o) = orders().lock().unwrap().get_mut(&oid2) {
                o.per_entry = outs.clone();
            }
        }
        let result = json!({
            "kind": "result", "ts": now_epoch(), "order_id": oid2,
            "per_entry": outs.iter().map(|e| json!({
                "name": e.name, "outcome": e.outcome, "reason_code": e.reason_code,
                "boundary": e.boundary, "bytes_before": e.bytes_before,
                "duration_ms": e.duration_ms, "remaining": e.remaining, "errors": e.errors,
            })).collect::<Vec<_>>(),
        });
        let _ = ledger_append(&state2, &result);
        if let Some(o) = orders().lock().unwrap().get_mut(&oid2) {
            o.per_entry = outs;
            o.finished = Some(now_epoch());
        }
        deleter_release();
    });
    Reply::Single(ok_response(id, json!({"order_id": order_id})))
}

fn str_list(params: &Value, key: &str) -> Option<Vec<String>> {
    params.get(key)?.as_array().map(|a| {
        a.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    })
}

/// 휴지통 뿌리의 **직속 자식 이름**만 받는다 — 경로 구분자·`..`·절대경로는 전부 거부.
pub fn safe_name(n: &str) -> bool {
    !n.is_empty()
        && n != "."
        && n != ".."
        && !n.contains('/')
        && !n.contains('\\')
        && !n.contains('\0')
}

fn new_id(prefix: &str) -> String {
    let mut h = Sha256::new();
    h.update(prefix.as_bytes());
    h.update(now_epoch().to_bits().to_le_bytes());
    h.update(std::process::id().to_le_bytes());
    h.update(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    format!("{prefix}-{}", &hex32(h.finalize().as_slice())[..16])
}

// ─── D-r2-5 M3: `.producer` 표식 **읽기** — 생산자가 돌고 있으면 거부한다 ─────────────────────
/// 표식 상태 판정(순수 · 주입 가능). `alive` 는 pid+start_token 생존 판정의 결과다.
#[derive(Debug, Clone, PartialEq)]
pub enum ProducerState {
    /// 표식 없음 — 구 버전이나 손으로 만든 항목(M1·M2 만으로 보호된다).
    Unknown,
    /// 돌고 있다 → **거부**(`producer_active`).
    ActiveOpen,
    /// 열린 채 죽었다 → 지울 수 있지만 "생산자가 끝내지 못함(중단)" 표지를 띄운다.
    InterruptedOpen,
    /// 정상 종료(complete·aborted).
    Settled,
    /// 생존 판정 불가 → **살아 있는 것으로 취급**(fail closed).
    Undeterminable,
}

/// `<entry>/.producer` 를 읽어 상태를 돌려준다. 파일이 깨졌으면 Undeterminable(= 살아 있는 쪽).
pub fn producer_state(entry: &Path, alive: &dyn Fn(u32, &str) -> Option<bool>) -> ProducerState {
    let p = entry.join(".producer");
    let txt = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return ProducerState::Unknown,
        Err(_) => return ProducerState::Undeterminable,
    };
    let v: Value = match serde_json::from_str(&txt) {
        Ok(v) => v,
        Err(_) => return ProducerState::Undeterminable,
    };
    match v.get("state").and_then(Value::as_str) {
        Some("complete") | Some("aborted") => ProducerState::Settled,
        Some("open") => {
            let pid = v.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32;
            let tok = v.get("start_token").and_then(Value::as_str).unwrap_or("");
            match alive(pid, tok) {
                Some(true) => ProducerState::ActiveOpen,
                Some(false) => ProducerState::InterruptedOpen,
                None => ProducerState::Undeterminable,
            }
        }
        _ => ProducerState::Undeterminable,
    }
}

/// 운영 기본 생존 판정 — **pid 존재만 본다.**
/// ★정정(구현을 다시 읽고 고친 주석): `start_token` 은 **대조하지 않는다**(인자 이름이 `_` 로
/// 시작하는 이유다). 종전 주석은 "토큰을 대조할 수 없으면 판정 불가를 돌려준다"고 적었는데 코드는
/// 그렇게 하지 않았다 — 그리고 그렇게 바꾸면 설계 I-5·I-9 의 "open + 죽은 pid → 중단 표지와 함께
/// 삭제 가능" 경로가 영구히 사라진다(토큰이 실린 모든 표식이 '살아 있음'이 되므로). 그래서 **주석을
/// 코드에 맞췄다.** 남는 한계는 정직하게 적는다: **pid 재사용**은 '살아 있다'로 오판될 수 있고,
/// 그 오판의 방향은 거부(안전)다. 세 writer 는 토큰을 기록만 해 둔다(나중에 대조할 수 있게).
/// 윈도우에서는 `kill(0)` 이 없어 판정 불가다(★윈도우에서 다르게 도는 자리 3).
pub fn real_alive(pid: u32, _start_token: &str) -> Option<bool> {
    if pid == 0 {
        return None;
    }
    #[cfg(unix)]
    {
        // signal 0 = 존재 확인만. 권한 오류(EPERM)는 "있다"는 뜻이다.
        let rc = unsafe { libc::kill(pid as i32, 0) };
        if rc == 0 {
            return Some(true);
        }
        match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::EPERM) => Some(true),
            Some(libc::ESRCH) => Some(false),
            _ => None,
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// 생산자 상태가 삭제를 막는가 — `ActiveOpen` 과 `Undeterminable` 은 막는다(fail closed).
pub fn producer_blocks(st: &ProducerState) -> bool {
    matches!(st, ProducerState::ActiveOpen | ProducerState::Undeterminable)
}

/// 화면 표지(칩) 문구 — 상태별로 하나.
pub fn producer_chip(st: &ProducerState) -> &'static str {
    match st {
        ProducerState::Unknown => "producer unknown (older version or hand-made)",
        ProducerState::ActiveOpen => "producer is running",
        ProducerState::InterruptedOpen => "producer did not finish (interrupted)",
        ProducerState::Settled => "",
        ProducerState::Undeterminable => "producer liveness undeterminable (treated as running)",
    }
}

/// ★안전 방향 전용 kill switch(S8:239) — `CYS_TRASH_DELETE=off` 면 삭제만 끈다(목록·표시는 그대로).
/// 위험한 방향으로는 어떤 환경변수도 가지 않는다.
pub fn delete_killswitched() -> bool {
    std::env::var("CYS_TRASH_DELETE")
        .map(|v| v.trim().eq_ignore_ascii_case("off"))
        .unwrap_or(false)
}

/// 휴지통 뿌리의 건전성 — 심링크거나 디렉터리가 아니면 **동작하지 않는다**(S8:115).
pub fn root_sane(root: &Path) -> bool {
    match std::fs::symlink_metadata(root) {
        Ok(md) => md.is_dir() && !md.file_type().is_symlink(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 격리된 임시 뿌리. 시험은 **실제 휴지통을 건드리지 않는다**(살아 있는 자료 무접촉).
    fn rig(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "cys-s8-{tag}-{}-{}",
            std::process::id(),
            now_epoch() as u64
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("rig");
        p
    }
    fn mk(p: &Path, rel: &str, bytes: &[u8]) {
        let f = p.join(rel);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, bytes).unwrap();
    }
    fn ledger_lines(state: &Path) -> Vec<Value> {
        std::fs::read_to_string(state.join(DELETE_LEDGER))
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("원장 줄은 JSON 이어야 한다"))
            .collect()
    }

    // ───────────────────────────────────────────────────────────────────────────────────────
    // ★C-R4 KNOWN-FAILURE 픽스처 — 이 기계에는 휴지통 뿌리 아래 교차장치 디렉터리가 **0개**다
    //   (부서장 실측 · 내 재측정 2026-10-12 15:02 에도 cross_device_nodes=0). 그래서 초록은
    //   아무 것도 증명하지 못한다 — 장치 경계를 **의도적으로 만들어** 거부를 재는 시험이다.
    // ───────────────────────────────────────────────────────────────────────────────────────
    #[test]
    fn cr4_descendant_device_refuses_the_whole_entry_and_writes_a_refusal_record() {
        let root = rig("crossdev");
        let entry = root.join("dept-9-1760000000");
        mk(&entry, "state/keep.txt", b"sole copy");
        mk(&entry, "deep/mount/inside.txt", b"on another device");
        let state = root.join("state");
        std::fs::create_dir_all(&state).unwrap();
        // 경계에서 장치 번호를 주입한다(합성 마운트).
        let boundary = entry.join("deep/mount");
        let b2 = boundary.clone();
        let dev_of = move |p: &Path, md: &std::fs::Metadata| -> u64 {
            if p == b2.as_path() || p.starts_with(&b2) {
                999_999
            } else {
                real_dev(p, md)
            }
        };
        let r = certify_entry(&entry, &dev_of).expect_err("교차장치면 인증이 거부돼야 한다");
        assert_eq!(r.code, R_DESC_DEVICE, "사유 코드가 장치 경계여야 한다");
        assert_eq!(
            r.boundary.as_deref(),
            Some(boundary.display().to_string().as_str()),
            "거부는 인증을 무너뜨린 **경계 경로**를 들고 있어야 한다"
        );
        // ★항목 전체 거부 — 경계 바깥의 파일도 그대로 있어야 한다(부분 제거 금지).
        assert!(entry.join("state/keep.txt").exists(), "경계 밖 파일이 사라졌다 = 부분 제거");
        assert!(boundary.join("inside.txt").exists());
        // ★거부는 기록된다(C-R2).
        record_refusal(&state, "od-test", "dept-9-1760000000", &r).expect("거부 기록");
        let ls = ledger_lines(&state);
        assert_eq!(ls.len(), 1);
        assert_eq!(ls[0]["kind"], "refusal");
        assert_eq!(ls[0]["reason_code"], R_DESC_DEVICE);
        assert_eq!(ls[0]["boundary"], boundary.display().to_string());
        assert_eq!(ls[0]["human_verified"], false, "원장은 사람을 증명하지 않는다");
        assert_eq!(ls[0]["identity"], "token_holder");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 집행 경로까지 포함한 교차장치 거부 — **한 바이트도 지워지지 않는다**.
    #[test]
    fn cr4_execute_refuses_and_removes_nothing_when_a_descendant_device_appears() {
        let root = rig("crossdev-exec");
        let name = "dept-9-1760000000";
        let entry = root.join(name);
        mk(&entry, "state/keep.txt", b"sole copy");
        mk(&entry, "deep/mount/inside.txt", b"other device");
        let state = root.join("state-dir");
        std::fs::create_dir_all(&state).unwrap();
        // 미리보기는 장치가 하나인 세계에서 통과했다고 가정한다(기준 인증서).
        let base = certify_entry(&entry, &real_dev).expect("기준 인증");
        // 집행 시점에 경계가 나타난다(마운트가 그 사이에 생긴 경우).
        let b2 = entry.join("deep/mount");
        let dev_of = move |p: &Path, md: &std::fs::Metadata| -> u64 {
            if p.starts_with(&b2) { 424_242 } else { real_dev(p, md) }
        };
        let out = execute_one(&root, name, &base, &dev_of, now_epoch() + 100_000.0, &state, "od-x");
        assert_eq!(out.outcome, "refused");
        assert_eq!(out.reason_code.as_deref(), Some(R_DESC_DEVICE));
        assert!(entry.exists(), "항목이 봉인·제거돼선 안 된다");
        assert!(entry.join("state/keep.txt").exists(), "한 바이트도 지워지지 않아야 한다");
        assert!(!root.join(format!("{TRASH_SEAL_PREFIX}{name}")).exists(), "봉인조차 하지 않는다");
        let ls = ledger_lines(&state);
        assert!(ls.iter().any(|l| l["reason_code"] == R_DESC_DEVICE), "거부 기록이 있어야 한다");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ─── ADDENDUM-1 + r2 m1: 두 창의 경계 오라클(같음의 방향이 서로 다르다) ─────────────────
    #[test]
    fn a1_m1_recent_boundary_599_600_601() {
        let now = 1_800_000_000.0;
        let w = TRASH_RECENT_WINDOW_SECS; // 600
        assert!(is_recent(now - (w as f64 - 1.0), now, w), "599s = 최근 = 거부");
        assert!(!is_recent(now - w as f64, now, w), "600s = 최근 아님 = 허용(r2 m1 이 ADDENDUM-1 을 대체)");
        assert!(!is_recent(now - (w as f64 + 1.0), now, w), "601s = 허용");
    }
    #[test]
    fn a1_unknown_recency_is_recent_fail_closed() {
        let now = 1_800_000_000.0;
        let w = TRASH_RECENT_WINDOW_SECS;
        assert!(is_recent(now + 5_000.0, now, w), "미래 mtime = 최근 = 거부");
        assert!(is_recent(f64::NAN, now, w), "판독 불가(NaN) = 거부");
        assert!(is_recent(f64::INFINITY, now, w), "비유한 = 거부");
        assert!(is_recent(now - 10_000.0, f64::NAN, w), "지금을 모르면 = 거부");
    }
    #[test]
    fn a1_m1_confirm_ttl_boundary_equality_expires() {
        let t = TRASH_CONFIRM_TTL_SECS; // 300
        assert!(!confirm_expired(t - 1, t), "299s = 쓸 수 있다");
        assert!(confirm_expired(t, t), "300s = 만료(같음은 만료다)");
        assert!(confirm_expired(t + 1, t), "301s = 만료");
    }

    // ─── D-r2-9: 유예 시계는 이름의 스탬프다(생산자 두 형식 · 미지 = 오래되지 않음) ──────────
    #[test]
    fn r2_old_is_read_from_the_name_stamp_in_both_formats() {
        // ② 압축 UTC — 실 엔트리의 형식.
        let iso = stamp_epoch_from_name("factory-reset-20260912T090920Z").expect("ISO 스탬프");
        // 기대값은 독립 계산으로 확인했다(python datetime(2026,9,12,9,9,20,tz=utc).timestamp()
        // = 1789204160). 처음 손으로 적은 값은 16시간 어긋났고, 이 대조가 그것을 잡았다.
        assert_eq!(iso, 1_789_204_160.0, "2026-09-12T09:09:20Z 의 epoch");
        // ① epoch 초 — cys-dept·javis_org 의 기본값.
        assert_eq!(stamp_epoch_from_name("dept-3-1760000000"), Some(1_760_000_000.0));
        // 미지 = None = 오래되지 않음.
        assert_eq!(stamp_epoch_from_name("hand-made-folder"), None);
        assert!(!is_old_by_stamp("hand-made-folder", 9_999_999_999.0, 14.0), "미지는 절대 old 가 아니다");
        assert!(is_old_by_stamp("dept-3-1760000000", 1_760_000_000.0 + 15.0 * 86400.0, 14.0));
        assert!(!is_old_by_stamp("dept-3-1760000000", 1_760_000_000.0 + 13.0 * 86400.0, 14.0));
        // 미래 스탬프도 old 가 아니다.
        assert!(!is_old_by_stamp("dept-3-1760000000", 1_700_000_000.0, 14.0));
    }

    // ─── D-r2-3: 집합 요약은 순서 무관이고 내용이 바뀌면 바뀐다 ──────────────────────────────
    #[test]
    fn r2_set_digest_changes_with_content_and_identity_is_carried() {
        let root = rig("digest");
        let e = root.join("dept-1-1760000000");
        mk(&e, "a/one.txt", b"1");
        mk(&e, "b/two.txt", b"22");
        let c1 = certify_entry(&e, &real_dev).unwrap();
        assert_eq!(c1.nodes, 4, "a, a/one.txt, b, b/two.txt");
        assert!(c1.dev != 0 || cfg!(windows));
        // 깊은 곳에 한 줄 추가 — 뿌리 mtime 이 아니라 요약이 움직인다(E1 의 교훈).
        mk(&e, "b/three.txt", b"333");
        let c2 = certify_entry(&e, &real_dev).unwrap();
        assert_ne!(c1.digest, c2.digest, "깊은 변경이 요약에 반영돼야 한다");
        assert_eq!(c2.nodes, 5);
        let _ = std::fs::remove_dir_all(&root);
    }

    // ─── 집행: 정상·변경·이미 사라짐 ─────────────────────────────────────────────────────────
    #[test]
    fn execute_removes_only_the_previewed_set_and_verifies_with_lstat() {
        let root = rig("exec-ok");
        let name = "dept-2-1760000000";
        let e = root.join(name);
        mk(&e, "state/x.txt", b"x");
        let state = root.join("sd");
        std::fs::create_dir_all(&state).unwrap();
        let base = certify_entry(&e, &real_dev).unwrap();
        let out = execute_one(&root, name, &base, &real_dev, now_epoch() + 100_000.0, &state, "od-1");
        assert_eq!(out.outcome, "deleted_verified", "{out:?}");
        assert!(!e.exists(), "항목이 사라져야 한다");
        assert!(!root.join(format!("{TRASH_SEAL_PREFIX}{name}")).exists(), "봉인 잔재도 없어야 한다");
        assert_eq!(out.remaining, None);
        let _ = std::fs::remove_dir_all(&root);
    }
    #[test]
    fn execute_refuses_when_the_content_changed_after_the_preview() {
        let root = rig("exec-changed");
        let name = "dept-2-1760000000";
        let e = root.join(name);
        mk(&e, "state/x.txt", b"x");
        let state = root.join("sd");
        std::fs::create_dir_all(&state).unwrap();
        let base = certify_entry(&e, &real_dev).unwrap();
        mk(&e, "state/added-after-preview.txt", b"new");
        let out = execute_one(&root, name, &base, &real_dev, now_epoch() + 100_000.0, &state, "od-2");
        assert_eq!(out.outcome, "refused");
        assert_eq!(out.reason_code.as_deref(), Some(R_CHANGED));
        assert!(e.exists() && e.join("state/x.txt").exists(), "아무 것도 지워지지 않아야 한다");
        let _ = std::fs::remove_dir_all(&root);
    }
    #[test]
    fn execute_refuses_a_recent_entry_even_after_a_clean_preview() {
        let root = rig("exec-recent");
        let name = "dept-2-1760000000";
        let e = root.join(name);
        mk(&e, "state/x.txt", b"x");
        let state = root.join("sd");
        std::fs::create_dir_all(&state).unwrap();
        let base = certify_entry(&e, &real_dev).unwrap();
        // 지금 막 쓴 파일이므로 '지금'을 실제 now 로 주면 최근이다.
        let out = execute_one(&root, name, &base, &real_dev, now_epoch(), &state, "od-3");
        assert_eq!(out.reason_code.as_deref(), Some(R_RECENT), "{out:?}");
        assert!(e.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
    #[test]
    fn execute_says_already_gone_without_claiming_a_deletion() {
        let root = rig("exec-gone");
        let name = "dept-2-1760000000";
        let e = root.join(name);
        mk(&e, "state/x.txt", b"x");
        let state = root.join("sd");
        std::fs::create_dir_all(&state).unwrap();
        let base = certify_entry(&e, &real_dev).unwrap();
        std::fs::remove_dir_all(&e).unwrap();
        let out = execute_one(&root, name, &base, &real_dev, now_epoch() + 100_000.0, &state, "od-4");
        assert_eq!(out.outcome, "already_gone");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ─── D-r2-5 M3 표식 읽기 ─────────────────────────────────────────────────────────────────
    #[test]
    fn producer_marker_states_and_what_they_block() {
        let root = rig("producer");
        let e = root.join("dept-5-1760000000");
        std::fs::create_dir_all(&e).unwrap();
        assert_eq!(producer_state(&e, &|_, _| Some(true)), ProducerState::Unknown);
        assert!(!producer_blocks(&ProducerState::Unknown), "표식 없음은 막지 않는다(구 생산자)");
        std::fs::write(e.join(".producer"), br#"{"state":"open","pid":4242,"start_token":"t"}"#).unwrap();
        assert_eq!(producer_state(&e, &|_, _| Some(true)), ProducerState::ActiveOpen);
        assert!(producer_blocks(&ProducerState::ActiveOpen), "돌고 있으면 막는다");
        assert_eq!(producer_state(&e, &|_, _| Some(false)), ProducerState::InterruptedOpen);
        assert!(!producer_blocks(&ProducerState::InterruptedOpen));
        assert_eq!(producer_state(&e, &|_, _| None), ProducerState::Undeterminable);
        assert!(producer_blocks(&ProducerState::Undeterminable), "판정 불가는 살아 있는 쪽(fail closed)");
        std::fs::write(e.join(".producer"), b"{ not json").unwrap();
        assert_eq!(producer_state(&e, &|_, _| Some(false)), ProducerState::Undeterminable);
        std::fs::write(e.join(".producer"), br#"{"state":"complete"}"#).unwrap();
        assert_eq!(producer_state(&e, &|_, _| Some(true)), ProducerState::Settled);
        let _ = std::fs::remove_dir_all(&root);
    }

    // ─── 이름·뿌리 건전성 ────────────────────────────────────────────────────────────────────
    #[test]
    fn only_direct_children_of_the_root_are_addressable() {
        assert!(safe_name("dept-1-1760000000"));
        for bad in ["", ".", "..", "a/b", "..\\b", "x\0y", "/abs"] {
            assert!(!safe_name(bad), "{bad:?} 는 거부돼야 한다");
        }
    }
    #[test]
    fn a_symlinked_or_nondir_root_is_refused() {
        let base = rig("rootsane");
        let real = base.join("real");
        std::fs::create_dir_all(&real).unwrap();
        assert!(root_sane(&real));
        let file = base.join("afile");
        std::fs::write(&file, b"x").unwrap();
        assert!(!root_sane(&file), "파일이면 동작하지 않는다");
        #[cfg(unix)]
        {
            let link = base.join("link");
            std::os::unix::fs::symlink(&real, &link).unwrap();
            assert!(!root_sane(&link), "심링크면 동작하지 않는다");
        }
        assert!(!root_sane(&base.join("missing")));
        let _ = std::fs::remove_dir_all(&base);
    }

    // ─── 조언 표지는 조언이다 ────────────────────────────────────────────────────────────────
    #[test]
    fn sole_copy_flag_is_advisory_and_errs_toward_sole() {
        assert!(sole_copy_advisory("factory-reset-20260912T090920Z", &[]));
        assert!(sole_copy_advisory("dept-4-1", &["state".into(), "pack".into()]));
        assert!(sole_copy_advisory("dept-4-1", &["pack-dept-dept-4".into()]));
        assert!(!sole_copy_advisory("dept-4-1", &["workdir".into()]));
    }

    // ─── 목록은 걷지 않는다 ─────────────────────────────────────────────────────────────────
    #[test]
    fn list_does_not_walk_and_reports_pending_sizes() {
        let root = rig("list");
        let e = root.join("dept-7-1760000000");
        mk(&e, "state/deep/a.txt", b"a");
        mk(&e, "pack/b.txt", b"b");
        let state = root.join("sd");
        std::fs::create_dir_all(&state).unwrap();
        let v = list_payload(&root, &state);
        let ents = v["entries"].as_array().unwrap();
        assert_eq!(ents.len(), 2, "뿌리의 직속 자식만 — state-dir 도 하나로 보인다");
        let me = ents.iter().find(|e| e["name"] == "dept-7-1760000000").unwrap();
        assert_eq!(me["size"]["state"], "pending", "측정 전에는 pending — 목록이 걷지 않는 증거");
        let kids = me["children_depth1"].as_array().unwrap();
        assert_eq!(kids.len(), 2, "깊이 1 만 — 'deep' 은 보이지 않는다");
        assert_eq!(me["sole_copy_advisory"], true);
        assert_eq!(v["constants"]["recent_window_secs"], 600);
        assert_eq!(v["constants"]["confirm_ttl_secs"], 300);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// ★성찰 A-2 의 수정: 점 파일(`.producer`)은 **오너가 보는 첫 단계 목록에 섞이지 않는다**
    /// (설계 r2 §3.4). 표식을 쓰기 시작한 뒤 생긴 결함이라 시험이 그 조합을 그대로 만든다.
    /// ★성찰 A-1 의 수정: 같은 줄의 `producer_chip` 과 `deletable` 은 **한 번의 판독**에서 나와야
    /// 서로 모순될 수 없다 — 아래 두 경우가 그 일치를 잠근다(살아 있는 pid / 죽은 pid).
    #[test]
    fn list_hides_dot_files_and_never_contradicts_itself_about_the_producer() {
        let root = rig("dotfiles");
        let state = root.join("sd");
        std::fs::create_dir_all(&state).unwrap();

        // (가) 살아 있는 생산자(내 pid) → 표지 "돌고 있다" ∧ 고를 수 없음
        let live = root.join("dept-9-1760000000");
        mk(&live, "state/x.txt", b"x");
        std::fs::write(
            live.join(".producer"),
            format!(
                r#"{{"v":1,"state":"open","producer":"t","pid":{},"start_token":"","ts":1}}"#,
                std::process::id()
            ),
        )
        .unwrap();
        // (나) 끝내지 못한 생산자(없는 pid) → 표지 "끝내지 못함" ∧ 고를 수 있음
        let dead = root.join("dept-10-1760000000");
        mk(&dead, "state/y.txt", b"y");
        std::fs::write(
            dead.join(".producer"),
            r#"{"v":1,"state":"open","producer":"t","pid":4294967294,"start_token":"","ts":1}"#,
        )
        .unwrap();

        let v = list_payload(&root, &state);
        let ents = v["entries"].as_array().unwrap();
        let g = ents.iter().find(|e| e["name"] == "dept-9-1760000000").unwrap();
        let kids = g["children_depth1"].as_array().unwrap();
        assert_eq!(kids.len(), 1, "점 파일이 첫 단계 목록에 섞였다: {kids:?}");
        assert_eq!(kids[0], "state");
        assert_eq!(g["children_total"], 1, "개수가 점 파일을 센다");
        assert_eq!(g["producer_chip"], producer_chip(&ProducerState::ActiveOpen));
        assert_eq!(g["deletable"], false, "돌고 있다고 적으면서 고를 수 있게 두면 모순이다");

        let d = ents.iter().find(|e| e["name"] == "dept-10-1760000000").unwrap();
        assert_eq!(d["producer_chip"], producer_chip(&ProducerState::InterruptedOpen));
        assert_eq!(d["deletable"], true, "끝내지 못한 생산자는 타이핑 확인 뒤 지울 수 있다");
        assert_eq!(d["children_depth1"].as_array().unwrap().len(), 1);

        // 판독은 항목마다 한 번이다 — 두 번 읽으면 두 값이 서로 다른 순간을 볼 수 있다.
        let src = include_str!("trash.rs");
        let body = &src[src.find("pub fn list_payload").unwrap()..src.find("fn ttl_days_default").unwrap()];
        assert_eq!(
            body.matches("producer_state(&p, &real_alive)").count(),
            1,
            "list_payload 이 표식 상태를 두 번 읽는다(A-1 회귀)"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // ─── 봉인 잔재는 자동 재개되지 않는다 ───────────────────────────────────────────────────
    #[test]
    fn a_sealed_leftover_is_listed_and_never_resumed() {
        let root = rig("sealed");
        let name = "dept-8-1760000000";
        let sealed = root.join(format!("{TRASH_SEAL_PREFIX}{name}"));
        mk(&sealed, "state/x.txt", b"x");
        let state = root.join("sd");
        std::fs::create_dir_all(&state).unwrap();
        let v = list_payload(&root, &state);
        let me = v["entries"].as_array().unwrap().iter()
            .find(|e| e["name"].as_str().unwrap().starts_with(TRASH_SEAL_PREFIX)).unwrap().clone();
        assert_eq!(me["interrupted_deleting"], true);
        assert_eq!(me["deletable"], false, "중단된 봉인은 선택 대상이 아니다");
        // 같은 이름으로 다시 명령해도 봉인이 있으면 거부다.
        let e2 = root.join(name);
        mk(&e2, "state/x.txt", b"x");
        let base = certify_entry(&e2, &real_dev).unwrap();
        let out = execute_one(&root, name, &base, &real_dev, now_epoch() + 100_000.0, &state, "od-5");
        assert_eq!(out.reason_code.as_deref(), Some(R_SEAL_FAILED));
        assert!(sealed.exists() && e2.exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// ★봉인 뒤 대조의 하중 시험 — 봉인과 대조 사이에 파일이 하나 들어오면 **제거하지 않고**
    /// 봉인 상태로 멈춘다(되돌리지 않는다). 변이 M5(대조 제거)가 이 시험을 빨갛게 만든다.
    #[test]
    fn a_change_between_the_seal_and_the_compare_stops_in_the_sealed_state() {
        let root = rig("after-seal");
        let name = "dept-6-1760000000";
        let e = root.join(name);
        mk(&e, "state/x.txt", b"x");
        let state = root.join("sd");
        std::fs::create_dir_all(&state).unwrap();
        let base = certify_entry(&e, &real_dev).unwrap();
        fn inject(sealed: &Path) {
            std::fs::write(sealed.join("state/snuck-in.txt"), b"after the seal").unwrap();
        }
        *AFTER_SEAL_HOOK.lock().unwrap() = Some((root.clone(), inject));
        let out = execute_one(&root, name, &base, &real_dev, now_epoch() + 100_000.0, &state, "od-seal");
        *AFTER_SEAL_HOOK.lock().unwrap() = None;
        assert_eq!(out.outcome, "sealed_not_removed", "{out:?}");
        assert_eq!(out.reason_code.as_deref(), Some(R_CHANGED));
        let sealed = root.join(format!("{TRASH_SEAL_PREFIX}{name}"));
        assert!(sealed.exists(), "봉인 상태로 남아야 한다");
        assert!(sealed.join("state/x.txt").exists(), "한 바이트도 지워지지 않아야 한다");
        assert!(sealed.join("state/snuck-in.txt").exists(), "끼어든 파일도 그대로다");
        assert!(!e.exists(), "되돌리지 않는다(옛 이름으로 돌아가지 않는다)");
        let ls = ledger_lines(&state);
        assert!(ls.iter().any(|l| l["reason_code"] == R_CHANGED));
        let _ = std::fs::remove_dir_all(&root);
    }

    // ─── 핀: 에이전트가 닿을 CLI 경로가 없다 ────────────────────────────────────────────────
    #[test]
    fn no_cli_verb_reaches_the_trash_rpcs() {
        let cys = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bin/cys.rs"),
        )
        .expect("cys.rs");
        for needle in ["trash.delete", "trash.preview", "trash.order", "trash.list"] {
            assert!(!cys.contains(needle), "cys.rs 에 {needle} 가 있다 — 에이전트 경로가 열린다");
        }
    }
    #[test]
    fn the_windows_validation_feature_is_not_a_default_feature() {
        let toml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        )
        .expect("Cargo.toml");
        assert!(toml.contains("trash_win_validated = []"), "feature 선언이 없다");
        let feats = toml.split("[features]").nth(1).expect("[features] 절");
        let default_line = feats
            .lines()
            .find(|l| l.trim_start().starts_with("default ="))
            .expect("default 줄");
        assert!(
            !default_line.contains("trash_win_validated"),
            "검증 feature 가 default 에 들어갔다 — 미검증 윈도우 삭제가 켜진다"
        );
    }
    /// ADDENDUM-1 의 grep 핀 — 이 모듈 안에 두 창의 리터럴이 상수 정의 밖에서 나타나지 않는다.
    #[test]
    fn the_two_windows_are_named_constants_with_no_other_literal_in_this_module() {
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bin/cysd/trash.rs"),
        )
        .expect("trash.rs");
        for (lit, konst) in [("600", "TRASH_RECENT_WINDOW_SECS"), ("300", "TRASH_CONFIRM_TTL_SECS")] {
            for (i, line) in src.lines().enumerate() {
                let t = line.trim_start();
                if t.starts_with("//") || t.starts_with("///") || t.starts_with("//!") {
                    continue;
                }
                if line.contains(konst) || line.contains("#[test]") {
                    continue;
                }
                // 시험 본문의 경계 계산은 상수에서 파생되므로 리터럴이 없어야 한다.
                assert!(
                    !line.contains(&format!("= {lit}")) || line.contains(&format!("{konst}: u64 = {lit}")),
                    "{}행에 {lit} 리터럴이 상수 정의 밖에서 있다: {line}",
                    i + 1
                );
            }
        }
    }
}
