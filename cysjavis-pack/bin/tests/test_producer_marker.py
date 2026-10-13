#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""test_producer_marker.py — S8 M3 핀: `cys-dept` 의 `.producer` 표식 쓰기.

읽는 쪽(데몬 `trash.rs producer_state`)의 계약을 **쓰는 쪽**에서 지키는지 본다:
  ① open 은 `{"v":1,"state":"open","pid":<내 pid>,...}` 를 **파싱 가능한 JSON** 으로 남긴다
     (데몬은 serde_json 으로 읽고, 깨진 파일은 '판정 불가 = 살아 있는 쪽'으로 취급해 삭제를 막는다).
  ② complete 로 닫힌다.
  ③ ★남의 창은 덮지 않고 남의 표식은 닫지 않는다 — 한 격리 폴더를 생산자 둘(cys-dept·javis_org.py)이
     쓰므로 표식 파일도 하나다.
  ④ 닫기는 폴더를 만들지 않는다(표식 때문에 빈 격리 항목이 생기면 그 자체가 새 쓰레기다).
  ⑤ ★쓰기가 실패해도 **격리를 실패·지연·재배열시키지 않는다**: stderr 한 줄, 종료 코드 0, 임시파일 0.

밀폐 하네스: 임시 디렉터리에서만 돌고 실 휴지통(`~/.local/state/cys-trash`)·실 데몬을 건드리지
않는다(부서 다섯 원칙 ⑤ 재현은 시험 자리에서만). `cys-dept` 명령 자체는 이 좌석의 역할 게이트가
거부하므로, 시험은 **제품 파일에서 떼어 온 그 함수의 실제 바이트**를 격리 실행한다 —
증명 범위는 "이 함수가 이렇게 쓴다"이고 "down --purge-state 전체가 그렇다"가 아니다(한계 명시).
"""
import json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
DEPT = os.path.join(HERE, "..", "cys-dept")
FAIL = []


def check(cond, msg):
    if not cond:
        FAIL.append(msg)
        print("  FAIL:", msg)
    else:
        print("  ok  :", msg)


def extract_fn():
    """제품 파일에서 `producer_mark()` 의 실제 바이트를 떼어 온다(손으로 베낀 사본이 아니다)."""
    src = open(DEPT, encoding="utf-8").read().splitlines()
    start = next(i for i, l in enumerate(src) if l.startswith("producer_mark(){"))
    end = next(i for i, l in enumerate(src[start:], start) if l == "}")
    return "\n".join(src[start:end + 1])


FN = extract_fn()


def run(script, env=None):
    e = dict(os.environ)
    e.update(env or {})
    return subprocess.run(["bash", "-c", FN + "\n" + script], capture_output=True, text=True, env=e)


def read_mark(d):
    try:
        with open(os.path.join(d, ".producer"), encoding="utf-8") as f:
            return json.load(f)   # 데몬과 같은 길: 파싱이 안 되면 그것이 결함이다
    except Exception as exc:
        return {"_parse_error": str(exc)}


def main():
    print("== S8 M3 `.producer` 표식 핀 ==")
    with tempfile.TemporaryDirectory() as td:
        # ① open — 파싱 가능한 JSON · 내 pid · v=1 · 생산자 이름
        d1 = os.path.join(td, "dept-x-100")
        os.makedirs(d1)
        # ★한 프로세스 안에서 열고 닫는다 — 실 흐름이 그렇고(같은 cys-dept 가 둘 다 한다),
        #   `$$` 가 소유 판정의 기준이라 호출을 두 프로세스로 쪼개면 둘째가 **남의 표식**이 된다
        #   (시험을 처음 그렇게 썼다가 ②가 빨개져 이 사실을 알았다 — 소유 규칙이 실제로 작동한다는
        #   증거이기도 하다).
        r = run('producer_mark "{d}" open; cp "{d}/.producer" "{d}/../open-snapshot"; '
                'producer_mark "{d}" complete; echo "PID=$$"'.format(d=d1))
        pid = int(r.stdout.strip().split("PID=")[1])
        with open(os.path.join(td, "open-snapshot"), encoding="utf-8") as f:
            opened = json.load(f)
        m = read_mark(d1)
        check(r.returncode == 0, "open: 종료 코드 0 (got %d / %s)" % (r.returncode, r.stderr.strip()))
        check(opened.get("v") == 1 and opened.get("state") == "open" and opened.get("pid") == pid
              and opened.get("producer") == "cys-dept purge-state" and "start_token" in opened
              and isinstance(opened.get("ts"), int),
              "open: 계약 필드 전부 · JSON 파싱 가능 — %r" % opened)
        check(not [f for f in os.listdir(d1) if f.startswith(".producer.tmp-")], "open: 임시파일 잔존 0")

        # ② complete 로 닫힌다(같은 프로세스)
        check(m.get("state") == "complete" and m.get("pid") == pid,
              "complete: 내 표식은 내가 닫는다 — %r" % m)

        # ③ 남의 창은 덮지 않고, 남의 표식은 닫지 않는다 (pid 1 = 언제나 살아 있다)
        d2 = os.path.join(td, "dept-y-100")
        os.makedirs(d2)
        foreign = {"v": 1, "state": "open", "producer": "javis_org.destroy_dept", "pid": 1,
                   "start_token": "", "ts": 1.0}
        with open(os.path.join(d2, ".producer"), "w", encoding="utf-8") as f:
            json.dump(foreign, f)
        run('producer_mark "%s" open' % d2)
        check(read_mark(d2) == foreign, "ownership: 다른 pid 의 open 창을 덮지 않는다 — %r" % read_mark(d2))
        run('producer_mark "%s" complete' % d2)
        check(read_mark(d2) == foreign, "ownership: 남의 표식을 닫지 않는다 — %r" % read_mark(d2))

        # ④ 닫기는 폴더를 만들지 않는다 / create=1 은 만든다(이동이 확정된 순간에만 쓰인다)
        d3 = os.path.join(td, "never")
        run('producer_mark "%s" complete' % d3)
        check(not os.path.exists(d3), "닫기는 격리 폴더를 만들지 않는다")
        d4 = os.path.join(td, "created")
        run('producer_mark "%s" open 1' % d4)
        check(os.path.isdir(d4) and read_mark(d4).get("state") == "open",
              "create=1: 폴더를 만들고 창을 연다 — %r" % read_mark(d4))

        # ⑤ 쓸 수 없어도 격리를 방해하지 않는다 — stderr 한 줄 · 종료 코드 0 · 임시파일 0
        d5 = os.path.join(td, "readonly")
        os.makedirs(d5)
        os.chmod(d5, 0o500)
        try:
            r = run('producer_mark "%s" open; echo "RC=$?"' % d5)
            check(r.returncode == 0 and "RC=0" in r.stdout,
                  "쓰기 불가: 함수가 0 으로 돌아온다(호출부 흐름 불변) — %s" % r.stdout.strip())
            check("표식 기록 실패(격리는 그대로 계속한다)" in r.stderr,
                  "쓰기 불가: stderr 한 줄로 정직 보고 — %r" % r.stderr.strip())
            os.chmod(d5, 0o700)
            check(os.listdir(d5) == [], "쓰기 불가: 임시파일 잔존 0 — %r" % os.listdir(d5))
        finally:
            os.chmod(d5, 0o700)

        # ⑥ ★표식 자리가 디렉터리인 경우 — `mv -f tmp dir` 는 **디렉터리 안으로 옮기고 성공**한다.
        #    그래서 가드가 없으면 표식은 안 써지는데 함수는 성공으로 보이고 쓰레기 파일이 남는다.
        #    (이 경우는 변이 검산에서 발견했다: tmp 정리 코드를 지워도 시험이 초록이어서 '그 경로를
        #     아무 시험도 밟지 않는다'는 것을 알았고, 밟는 길을 찾다가 이 자리를 찾았다.)
        d6 = os.path.join(td, "mark-is-dir")
        os.makedirs(os.path.join(d6, ".producer"))
        r = run('producer_mark "%s" open; echo "RC=$?"' % d6)
        inside = os.listdir(os.path.join(d6, ".producer"))
        check(r.returncode == 0 and "RC=0" in r.stdout, "표식 자리가 디렉터리: 함수는 0 으로 돌아온다")
        check(inside == [], "표식 자리가 디렉터리: 그 안에 임시파일을 떨어뜨리지 않는다 — %r" % inside)
        check("표식 기록 실패(격리는 그대로 계속한다)" in r.stderr and "디렉터리다" in r.stderr,
              "표식 자리가 디렉터리: stderr 한 줄로 정직 보고 — %r" % r.stderr.strip())
        check(not [f for f in os.listdir(d6) if f.startswith(".producer.tmp-")],
              "표식 자리가 디렉터리: 격리 폴더에도 임시파일 0")

        # ⑦ ★tmp 는 써졌는데 교체가 실패하는 경우 — 임시파일을 남기지 않는다.
        #    (변이 검산에서 `rm -f "$tmp"` 를 지워도 초록이어서 '그 경로를 밟는 시험이 없다'는 것을
        #     알았다. 밟는 길: BSD/macOS 의 불변 플래그 `chflags uchg` — 폴더는 쓸 수 있으니 tmp 는
        #     써지고, 교체만 'Operation not permitted' 로 실패한다. 리눅스에는 이 플래그가 없어
        #     건너뛰며, 건너뛴 사실을 출력한다 — 조용히 통과시키지 않는다.)
        if sys.platform == "darwin" and subprocess.run(["which", "chflags"],
                                                       capture_output=True).returncode == 0:
            d7 = os.path.join(td, "mark-immutable")
            os.makedirs(d7)
            f7 = os.path.join(d7, ".producer")
            with open(f7, "w", encoding="utf-8") as f:
                # ★`complete` 로 둔다: 다른 pid 의 `complete` 는 인수 가능하므로(소유 규칙) 쓰기까지
                #   진행되고, 그래야 '교체 실패' 경로를 실제로 밟는다. `open` 으로 두면 소유 규칙이
                #   먼저 막아 쓰기 자체가 없다(처음 그렇게 써서 stderr 가 비었고, 그래서 알았다).
                f.write('{"v":1,"state":"complete","producer":"x","pid":%d,"start_token":"","ts":1}' % os.getpid())
            subprocess.run(["chflags", "uchg", f7], check=True)
            try:
                r = run('producer_mark "%s" open; echo "RC=$?"' % d7)
                leftovers = [f for f in os.listdir(d7) if f.startswith(".producer.tmp-")]
                check(r.returncode == 0 and "RC=0" in r.stdout,
                      "교체 실패: 함수는 0 으로 돌아온다 — %s" % r.stdout.strip())
                check(leftovers == [], "교체 실패: 임시파일을 지운다 — %r" % leftovers)
                check("표식 기록 실패(격리는 그대로 계속한다)" in r.stderr,
                      "교체 실패: stderr 한 줄 — %r" % r.stderr.strip())
            finally:
                subprocess.run(["chflags", "nouchg", f7])
        else:
            print("  skip: 교체 실패 경로(chflags uchg)는 이 플랫폼에서 구성 불가 — 미검증으로 남는다")

    print("RESULT:", "PASS" if not FAIL else "FAIL (%d)" % len(FAIL))
    return 1 if FAIL else 0


if __name__ == "__main__":
    sys.exit(main())
