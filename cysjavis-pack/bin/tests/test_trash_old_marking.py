#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""test_trash_old_marking.py — S8 D-1 핀: reap 의 휴지통 블록은 **지우지 않고 표시만** 한다.

밀폐 하네스: `HOME` 과 `CYS_DEPTS_JSON` 을 임시 디렉터리로 덮어 실 휴지통
(`~/.local/state/cys-trash`)·실 레지스트리·실 소켓을 통째로 격리한다. 실 데몬을 부르지 않고
실 격리분을 건드리지 않는다 — 재현은 시험 자리에서만(부서 다섯 원칙 ⑤).

  ① 양성 — TTL 초과 격리분: 블록이 TRASH-OLD 로 알리고 **엔트리는 그대로 남는다**(종전 코드는
     여기서 `rm -rf` 했다). 세어 둔 값은 `TRASH-OLD=1`.
  ★한계: `cys-dept reap` 명령 자체는 이 좌석에서 돌릴 수 없다(역할 게이트가 데몬 권위 역할로
     거부한다 — 실측). 그래서 ①②는 **제품 파일에서 떼어 온 그 블록의 실제 바이트**를 격리 실행한
     결과다. 명령 전체의 통합 동작은 CSO 좌석이나 CI 에서 확인할 **미실행 출하 게이트**다.
  ② 음성 — TTL 미만 격리분: TRASH-OLD 0건, 엔트리 그대로.
  ③ 핀 — 스크립트에 `rm -rf "$_td"` 도 `TRASH-EXPIRE` 토큰도 없다.
  ④ 무접촉 — 알린 뒤에도 격리분 **안의 파일 목록·내용이 바뀌지 않는다**(표식 파일을 남기지 않는다:
     그 쓰기는 미리보기와 명령 사이에 내용 집합을 바꿔 오너의 명령을 헛되게 거부시킨다).
"""
import os, subprocess, sys, tempfile, time, hashlib, json

HERE = os.path.dirname(os.path.abspath(__file__))
DEPT = os.path.join(HERE, "..", "cys-dept")
FAIL = []


def check(cond, msg):
    if not cond:
        FAIL.append(msg)
        print("  FAIL:", msg)
    else:
        print("  ok  :", msg)


def tree_fingerprint(root):
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for f in sorted(filenames):
            p = os.path.join(dirpath, f)
            with open(p, "rb") as fh:
                out.append((os.path.relpath(p, root), hashlib.sha256(fh.read()).hexdigest()))
    return sorted(out)


def extract_block():
    """★제품 파일에서 휴지통 블록의 **실제 바이트**를 떼어 온다(손으로 베낀 사본이 아니다).

    `cys-dept reap` 자체는 이 좌석에서 실행할 수 없다 — 역할 게이트가 데몬 권위 역할(worker-2)로
    거부한다("단일소유 강제: 부서 lifecycle mutation은 CSO/GUI 전용"). 그래서 기능 확인은 그 명령을
    돌리는 것이 아니라 **그 명령의 그 블록을 떼어 격리 실행**하는 것이다. 이 시험이 증명하는 것은
    "이 블록이 지우지 않고 알린다"이고, "reap 전체가 그렇다"가 아니다(한계 명시).
    """
    src = open(DEPT, encoding="utf-8").read().splitlines()
    start = next(i for i, l in enumerate(src) if "★S8 D-1" in l)
    end = next(i for i, l in enumerate(src[start:], start) if l.strip() == "fi")
    return "\n".join(src[start:end + 1])


BLOCK = extract_block()


def run_reap(home, dry):
    """떼어 온 블록만 격리 실행한다(제품의 바이트 그대로 · 변수만 주입)."""
    trash = os.path.join(home, ".local", "state", "cys-trash")
    script = "set -u\nTRASH_ROOT=%s\ndry=%d\n%s\necho \"[cys-dept] reap 완료(dry=$dry): TRASH-OLD=$nt\"\n" % (
        json.dumps(trash), 1 if dry else 0, BLOCK)
    env = dict(os.environ)
    env.update(HOME=home, CYS_TRASH_TTL_DAYS="14")
    r = subprocess.run(["bash", "-c", script], capture_output=True, text=True, env=env, timeout=120)
    return r.returncode, r.stdout + r.stderr


def mk_entry(trash, name, age_days):
    d = os.path.join(trash, name)
    os.makedirs(os.path.join(d, "state"), exist_ok=True)
    with open(os.path.join(d, "state", "transcripts.db"), "w", encoding="utf-8") as f:
        f.write("sole copy of a department's conversation memory")
    old = time.time() - age_days * 86400
    for p in (os.path.join(d, "state", "transcripts.db"), os.path.join(d, "state"), d):
        os.utime(p, (old, old))
    return d


def case(age_days, expect_marked):
    with tempfile.TemporaryDirectory(prefix="cys-s8-d1-") as home:
        trash = os.path.join(home, ".local", "state", "cys-trash")
        os.makedirs(trash)
        with open(os.path.join(home, "depts.json"), "w", encoding="utf-8") as f:
            json.dump({"depts": {}}, f)
        name = "dept-9-%d" % int(time.time())
        d = mk_entry(trash, name, age_days)
        before = tree_fingerprint(d)
        for dry in (True, False):
            rc, out = run_reap(home, dry)
            tag = "dry" if dry else "real"
            marked = ("TRASH-OLD %s" % d) in out
            check(marked == expect_marked,
                  "[%s · %sd] TRASH-OLD 알림 %s" % (tag, age_days, "있음" if expect_marked else "없음"))
            check("TRASH-OLD=%d" % (1 if expect_marked else 0) in out,
                  "[%s · %sd] 요약 줄 TRASH-OLD=%d" % (tag, age_days, 1 if expect_marked else 0))
            check("TRASH-EXPIRE" not in out, "[%s · %sd] 출력에 TRASH-EXPIRE 토큰 없음" % (tag, age_days))
            check(os.path.isdir(d), "[%s · %sd] ★엔트리가 그대로 있다(지워지지 않았다)" % (tag, age_days))
        check(tree_fingerprint(d) == before,
              "[%sd] ★격리분 안의 내용이 한 바이트도 바뀌지 않았다(표식 파일 없음)" % age_days)


print("== ① 양성: TTL(14d) 초과 30일 격리분 ==")
case(30, True)
print("== ② 음성: 3일 격리분 ==")
case(3, False)
print("== ③ 핀: 파괴 블록 부재 ==")
src = open(DEPT, encoding="utf-8").read()
check('rm -rf "$_td"' not in src, '스크립트에 rm -rf "$_td" 가 없다')
check("TRASH-EXPIRE" not in src, "스크립트에 TRASH-EXPIRE 토큰이 없다")
check("TRASH-OLD" in src, "스크립트에 TRASH-OLD 가 있다")
check("delete from Control Center > Trash" in src, "안내 문구가 있다(누가 아니라 이 도구가 무엇을 하는지)")
print()
if FAIL:
    print("FAILED %d건" % len(FAIL))
    for f in FAIL:
        print(" -", f)
    sys.exit(1)
print("PASS — 휴지통 블록은 표시만 하고 지우지 않는다(reap 명령 전체는 미실행 게이트)")
