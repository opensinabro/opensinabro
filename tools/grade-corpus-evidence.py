#!/usr/bin/env python3
"""문법 회귀 코퍼스(fixtures/corpus/)의 근거 등급을 도움말 원문과 대조해 검사한다.

근거 등급은 주장이 아니라 도출된 값이다. `도움말예제`는 "케이스 원문이 알파위키 문법
도움말에 그대로 실려 있다"는 뜻이고, 그래야 그 마크업의 the seed 렌더가 캐시에 실재해
`parity compare`로 대조할 수 있다. 이 스크립트가 그 대조를 대신한다.

`도움말서술`과 `미확인`은 기계적으로 가를 수 없다 — 도움말이 그 동작을 서술하는지는
사람이 읽고 판단한 것이다. 그래서 이 둘 사이의 강등·승격은 하지 않고, **도움말예제라고
적혀 있는데 도움말에 없는 것**과 **도움말예제가 될 수 있는데 아닌 것**만 보고한다.

사용법:

    python3 tools/fetch-parity-corpus.py        # 도움말 캐시를 먼저 받는다
    python3 tools/grade-corpus-evidence.py      # 검사 (어긋나면 종료 코드 1)

배경은 fixtures/corpus/README.md와 docs/design/04-namuwiki-parity.md 참고.
"""

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CORPUS = ROOT / "fixtures" / "corpus"
HELP_DOCUMENTS = [
    ROOT / "target" / "parity-corpus" / "알파위키_문법 도움말.namu",
    ROOT / "target" / "parity-corpus" / "알파위키_문법 도움말_심화.namu",
]


def help_source():
    missing = [path for path in HELP_DOCUMENTS if not path.exists()]
    if missing:
        print("도움말 캐시가 없습니다. 먼저 `python3 tools/fetch-parity-corpus.py`를 실행하세요.")
        print("\n".join(f"  없음: {path}" for path in missing))
        sys.exit(2)
    return "\n".join(path.read_text(encoding="utf-8") for path in HELP_DOCUMENTS)


def read_case(path):
    """첫 줄이 밝힌 줄 수만큼을 원문으로 떼어 낸다. 형식은 코퍼스 하네스와 같다."""
    header, body = path.read_text(encoding="utf-8").split("\n", 1)
    fields = header.split(" | ", 4)
    grade = fields[0].removeprefix("근거: ")
    source_lines = int(fields[1].removeprefix("원문: ").removesuffix("줄"))
    source = "".join(body.splitlines(keepends=True)[:source_lines])
    return grade, source


def main():
    help_text = help_source()
    wrong, promotable = [], []

    cases = sorted(CORPUS.glob("*/*.case"))
    for path in cases:
        grade, source = read_case(path)
        name = f"{path.parent.name}/{path.stem}"
        lines = [line.strip() for line in source.splitlines() if line.strip()]
        grounded = bool(lines) and all(line in help_text for line in lines)

        if grade == "도움말예제" and not grounded:
            wrong.append(name)
        elif grade == "도움말서술" and grounded:
            promotable.append(name)

    print(f"케이스 {len(cases)}건 검사")
    if wrong:
        print(f"\n도움말예제라고 적혔으나 도움말 원문에 없음 ({len(wrong)}건) — 등급을 낮추십시오:")
        print("\n".join(f"  {name}" for name in wrong))
    if promotable:
        print(f"\n도움말예제로 올릴 수 있음 ({len(promotable)}건) — 원문이 도움말에 그대로 있습니다:")
        print("\n".join(f"  {name}" for name in promotable))
    if not wrong and not promotable:
        print("모든 등급이 도움말 원문과 맞습니다.")
    return 1 if wrong else 0


if __name__ == "__main__":
    sys.exit(main())
