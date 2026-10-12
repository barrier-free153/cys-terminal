#!/usr/bin/env bash
# 격리 HOME E2E (기능2 부서 완전 폐역): cys-dept down --purge-state 격리·왕복복원 + javis_org destroy
# --purge-state 3디렉토리(state·pack·workdir) 동일 trash 격리 + 사후 검증기(javis_purge_verify.py)
# positive/negative fixture(위양성 검증기 차단). 라이브 데몬·라이브 depts.json 무접촉(격리 HOME·stub cys).
set -euo pipefail
BINDIR="$(cd "$(dirname "$0")" && pwd)"
T=$(mktemp -d)
export HOME="$T"
export CYS_PACK_DIR="$BINDIR/.."          # $ph=CYS_PACK_DIR/bin/javis_phoenix.py 해소(실 phoenix 묘비 기록)
export CYS_DEPTS_JSON="$T/.cys/depts.json"
export CYS_ROLE=cso CYS_TRASH_STAMP=222
STATE="$T/.local/state"
ROSTER="$STATE/cys/phoenix/dept_roster.json"
CYSDEPT="$BINDIR/cys-dept"
VERIFY="$BINDIR/javis_purge_verify.py"
ORG="$BINDIR/javis_org.py"

# stub cys/cysd (inert daemon RPC) — 라이브 데몬 무접촉. ★cys-dept 가 line 20 에서 $HOME/.local/bin·
# /opt/homebrew/bin 을 PATH 앞에 재-prepend 하므로, 실 cys(autostart cysd)가 새지 않게 stub 을
# **$HOME/.local/bin 에 배치**(cys-dept 자체 prepend 순서에서 최우선). CYS_NO_AUTOSTART 로 이중 방어.
export CYS_NO_AUTOSTART=1
mkdir -p "$T/.local/bin" "$T/bin"
cat > "$T/.local/bin/cys" <<'STUB'
#!/bin/bash
case "$1" in
  identify) echo '{}';; ping) exit 1;; status) echo '{"surfaces":[]}';;
  tombstone) echo '{"dept_tombstones":[]}';; *) exit 0;;
esac
STUB
printf '#!/bin/bash\nexit 0\n' > "$T/.local/bin/cysd"   # 실 cysd spawn 차단(nohup 스텁)
chmod +x "$T/.local/bin/cys" "$T/.local/bin/cysd"
cp "$T/.local/bin/cys" "$T/bin/cys"; chmod +x "$T/bin/cys"
export PATH="$T/.local/bin:$T/bin:$BINDIR:$PATH"   # stub 최우선, cys-dept 는 BINDIR 에서 해소

fail(){ echo "E2E FAIL: $1"; rm -rf "$T"; exit 1; }

setup_depts(){
  rm -rf "$STATE"; mkdir -p "$STATE/cys-dept-testdept" "$STATE/cys-dept-sibling" "$T/.cys"
  echo "CONVO-MEMORY" > "$STATE/cys-dept-testdept/transcripts.db"
  echo "SIB-MEMORY"   > "$STATE/cys-dept-sibling/transcripts.db"
  printf '' > "$STATE/cys-dept-testdept/cys.sock"; printf '' > "$STATE/cys-dept-sibling/cys.sock"
  cat > "$CYS_DEPTS_JSON" <<JSON
{"depts":{"testdept":{"socket":"$STATE/cys-dept-testdept/cys.sock","pack_dir":"$T/.cys/pack-dept-testdept","role":"dept-master"},"sibling":{"socket":"$STATE/cys-dept-sibling/cys.sock","pack_dir":"$T/.cys/pack-dept-sibling","role":"dept-master"}}}
JSON
}

echo "== 1) down testdept --purge-state (격리+묘비) + 사후 검증기 PASS =="
setup_depts
bash "$CYSDEPT" down testdept --purge-state >/dev/null 2>&1 || fail "down --purge-state rc!=0"
[ ! -d "$STATE/cys-dept-testdept" ] || fail "원위치 잔존(격리 미완)"
[ -f "$STATE/cys-trash/testdept-222/state/transcripts.db" ] || fail "격리 미보관(복구 불가)"
[ -f "$STATE/cys-dept-sibling/transcripts.db" ] || fail "형제 손상"
python3 "$VERIFY" --dept testdept --state-root "$STATE" --depts-json "$CYS_DEPTS_JSON" \
  --dept-roster "$ROSTER" --expect-sibling sibling >/dev/null || fail "검증기 happy-path FAIL(위음성)"
echo "  → PASS"

echo "== 2) NEGATIVE ① 불완전 purge(디렉토리 고의 잔존) → 검증기 non-zero FAIL =="
setup_depts
if python3 "$VERIFY" --dept testdept --state-root "$STATE" --dept-roster "$ROSTER" --no-tombstone >/dev/null; then
  fail "불완전 purge인데 검증기 PASS(위양성)"; fi
echo "  → 기대대로 FAIL"

echo "== 3) NEGATIVE ② 묘비 소실 → 검증기 non-zero FAIL =="
setup_depts
bash "$CYSDEPT" down testdept --purge-state >/dev/null 2>&1 || true
rm -f "$ROSTER"    # 묘비 파일 고의 제거
if python3 "$VERIFY" --dept testdept --state-root "$STATE" --dept-roster "$ROSTER" --expect-sibling sibling >/dev/null; then
  fail "묘비 소실인데 검증기 PASS(위양성)"; fi
echo "  → 기대대로 FAIL"

echo "== 4) NEGATIVE ③ 형제 소실 → 검증기 오염 탐지 non-zero FAIL =="
setup_depts
bash "$CYSDEPT" down testdept --purge-state >/dev/null 2>&1 || true
rm -rf "$STATE/cys-dept-sibling"    # 형제 고의 소실(무관 부서 손상 시뮬)
if python3 "$VERIFY" --dept testdept --state-root "$STATE" --dept-roster "$ROSTER" --expect-sibling sibling >/dev/null; then
  fail "형제 소실인데 검증기 PASS(오염 미탐지)"; fi
echo "  → 기대대로 FAIL"

echo "== 5) ④ 0-부서/부재 부서명 → 우아 exit(크래시 0) =="
rm -rf "$STATE"; mkdir -p "$STATE"
python3 "$VERIFY" --dept ghost --state-root "$STATE" --no-tombstone >/dev/null || fail "부재 부서명 검증기 크래시/FAIL"
bash "$CYSDEPT" down ghost --purge-state >/dev/null 2>&1 || fail "부재 부서 down --purge-state 크래시"
echo "  → 우아 처리"

echo "== 6) ⑤ 왕복: 격리분 복원 → 재창설 시 대화기억 접근 =="
setup_depts
bash "$CYSDEPT" down testdept --purge-state >/dev/null 2>&1 || true
[ ! -d "$STATE/cys-dept-testdept" ] || fail "격리 전제 위반"
mv "$STATE/cys-trash/testdept-222/state" "$STATE/cys-dept-testdept"    # 복원(재창설 시 데몬이 재사용)
grep -q CONVO-MEMORY "$STATE/cys-dept-testdept/transcripts.db" || fail "복원 후 대화기억 접근 불가"
echo "  → 복원·기억 접근 OK"

echo "== 7) javis_org destroy --purge-state (state+pack+workdir 동일 trash · workdir_owned 선언) =="
setup_depts
mkdir -p "$T/.cys/pack-dept-testdept/directives" "$T/work/testdept"
echo "PACK" > "$T/.cys/pack-dept-testdept/marker"
echo "WORK" > "$T/work/testdept/file.txt"
# ★D1a: workdir 격리는 소유권 선언(workdir_owned=true)이 있어야 적격 — opt-in 경로 검증
python3 -c "import json;p='$CYS_DEPTS_JSON';d=json.load(open(p));e=d['depts']['testdept'];e['cwd']='$T/work/testdept';e['workdir_owned']=True;json.dump(d,open(p,'w'))"
python3 "$ORG" destroy --dept testdept --purge --purge-workdir --purge-state >/dev/null 2>&1 || fail "javis_org destroy rc!=0"
TR="$STATE/cys-trash/testdept-222"
[ -f "$TR/state/transcripts.db" ] || fail "state 미격리"
[ -f "$TR/pack/marker" ]          || fail "pack 미격리"
[ -f "$TR/workdir/file.txt" ]     || fail "workdir 미격리"
[ ! -d "$STATE/cys-dept-testdept" ] || fail "state 원위치 잔존"
[ ! -d "$T/.cys/pack-dept-testdept" ] || fail "pack 원위치 잔존"
[ -f "$STATE/cys-dept-sibling/transcripts.db" ] || fail "형제 손상"
echo "  → 3디렉토리 동일 trash 격리 확인"

echo "== 7b) [D1a] 소유 미선언 workdir → 격리 skip(state·pack만 격리·destroy rc 0) =="
setup_depts
mkdir -p "$T/.cys/pack-dept-testdept" "$T/work/testdept"
echo "PACK" > "$T/.cys/pack-dept-testdept/marker"
echo "WORK-SHARED" > "$T/work/testdept/file.txt"
python3 -c "import json;p='$CYS_DEPTS_JSON';d=json.load(open(p));d['depts']['testdept']['cwd']='$T/work/testdept';json.dump(d,open(p,'w'))"
OUT7B=$(python3 "$ORG" destroy --dept testdept --purge --purge-workdir --purge-state 2>&1) || fail "[D1a] 미선언 skip인데 destroy rc!=0"
echo "$OUT7B" | grep -q "workdir_shared_skip" || fail "[D1a] skip 사유(workdir_shared_skip) 미기록"
[ -f "$T/work/testdept/file.txt" ] || fail "[D1a] 미선언 workdir이 격리됨(치명 — 공유 디렉토리 강탈)"
[ ! -d "$STATE/cys-trash/testdept-222/workdir" ] || fail "[D1a] trash에 workdir 생김(skip 위반)"
[ -f "$STATE/cys-trash/testdept-222/state/transcripts.db" ] || fail "[D1a] state는 격리됐어야 함"
echo "  → 기대대로 workdir 보존 + state·pack 격리 + rc 0"

echo "== 8) reap trash 만료 소거(TTL) =="
setup_depts
bash "$CYSDEPT" down testdept --purge-state >/dev/null 2>&1 || true
[ -d "$STATE/cys-trash/testdept-222" ] || fail "격리분 부재"
# mtime 을 20일 전으로 후퇴 → TTL 14일 초과
touch -t "$(date -v-20d +%Y%m%d%H%M 2>/dev/null || date -d '20 days ago' +%Y%m%d%H%M)" "$STATE/cys-trash/testdept-222"
bash "$CYSDEPT" reap >/dev/null 2>&1 || true
[ ! -d "$STATE/cys-trash/testdept-222" ] || fail "만료 격리분 미소거"
echo "  → 만료 소거 OK"

echo "== 9) [F1a] javis_org destroy: down 실패(exit3) 삼킴 방지 → destroy 비0+incomplete+state 잔존 =="
setup_depts
# stub cys-dept: down --purge-state 를 exit 3(state 격리 실패)로 시뮬(state 미이동). $T/.local/bin 우선.
cat > "$T/.local/bin/cys-dept" <<'STUBDEPT'
#!/bin/bash
if [ "$1" = "down" ]; then echo "[stub] down $* → state 격리 실패 시뮬" >&2; exit 3; fi
exit 0
STUBDEPT
chmod +x "$T/.local/bin/cys-dept"
# ★D1c: destroy가 cys-dept를 PATH가 아닌 자기 디렉토리에서 해소하므로 스텁은 CYS_DEPT_BIN으로 주입
OUT9=$(CYS_DEPT_BIN="$T/.local/bin/cys-dept" python3 "$ORG" destroy --dept testdept --purge --purge-workdir --purge-state 2>&1) && RC9=0 || RC9=$?
rm -f "$T/.local/bin/cys-dept"
[ "$RC9" != "0" ] || fail "[F1a] down exit3인데 destroy rc=0(삼킴 — 부분성공 오보)"
echo "$OUT9" | grep -q "incomplete" || fail "[F1a] destroy 출력에 incomplete 없음"
[ -d "$STATE/cys-dept-testdept" ] || fail "[F1a] state가 실제로는 안 옮겨졌는데 잔존 아님(전제 오류)"
echo "  → 기대대로 destroy 비0+incomplete(down 실패 정직 전파)"

echo "== 10) [F1b] 검증기 배선: down이 false-success(exit0·미격리)여도 사후 검증기가 포착 → destroy 비0 =="
setup_depts
# stub cys-dept: down --purge-state 를 exit 0 으로 보고하되 state 를 옮기지 않음(무음 false success).
cat > "$T/.local/bin/cys-dept" <<'STUBDEPT2'
#!/bin/bash
if [ "$1" = "down" ]; then echo "[stub] down $* → 성공 보고하나 미격리(false success)" >&2; exit 0; fi
exit 0
STUBDEPT2
chmod +x "$T/.local/bin/cys-dept"
OUT10=$(CYS_DEPT_BIN="$T/.local/bin/cys-dept" python3 "$ORG" destroy --dept testdept --purge --purge-workdir --purge-state 2>&1) && RC10=0 || RC10=$?
rm -f "$T/.local/bin/cys-dept"
[ "$RC10" != "0" ] || fail "[F1b] down false-success인데 destroy rc=0(검증기 미배선 — 부활창 오보)"
echo "$OUT10" | grep -qi "검증\|verify\|incomplete" || fail "[F1b] 검증 실패 신호 없음"
echo "  → 기대대로 destroy 비0(사후 검증기가 미격리 포착)"

echo "== 11) [F2] traversal 부서명 거부: down 'sibling/../x' --purge-state → 비0 + 네임스페이스 밖 무접촉 =="
# ★진짜 exploit 벡터: cys-dept-sibling(실재)/../x 는 pwd -P가 $STATE/x(네임스페이스 밖)로 탈출한다.
# 가드 없으면 mv $STATE/x → trash(대화기억 아닌 임의 디렉토리 강탈). 가드 있으면 진입부에서 거부.
setup_depts
mkdir -p "$STATE/x"; echo "OUTSIDE" > "$STATE/x/keep.txt"   # 네임스페이스 밖 표적
bash "$CYSDEPT" down 'sibling/../x' --purge-state >/dev/null 2>&1 && fail "[F2] traversal 부서명이 rc0(가드 부재 — mv 대참사 위험)" || true
[ -f "$STATE/x/keep.txt" ] || fail "[F2] 네임스페이스 밖 $STATE/x 가 격리됨(traversal 성공 — 치명)"
echo "  → 기대대로 거부(비0)+표적 무접촉"

echo "ALL PURGE E2E PASS (라이브 무접촉)"; rm -rf "$T"
