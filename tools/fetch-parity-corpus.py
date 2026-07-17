#!/usr/bin/env python3
"""알파위키(the seed 엔진)에서 파리티 대조용 코퍼스를 받아 로컬 캐시에 저장한다.

나무위키 본체는 크롤링을 막지만(403) 알파위키는 나무위키와 같은 the seed 엔진이고
원문과 렌더링 결과를 모두 공개한다. 근거·기법은 docs/design/04-namuwiki-parity.md 참고.

문서마다 두 파일을 만든다.

    <slug>.namu   원문 마크업      (GET /raw/<문서>)
    <slug>.html   the seed 렌더 본문 (GET /w/<문서>)

두 페이지 모두 본문이 HTML에 직접 있지 않고 `window.INITIAL_STATE`에
base64(zlib(protobuf))로 들어 있어 풀어야 한다. 렌더 본문은 문단별 조각으로 나뉘어
있으므로 protobuf 필드 순서대로 이어 붙인다.

사용법:

    python3 tools/fetch-parity-corpus.py                 # 기본 문서 목록
    python3 tools/fetch-parity-corpus.py "문서명" ...     # 문서 지정

`[include(틀:...)]`로 참조하는 틀 문서는 자동으로 함께 받는다(파리티 도구가
include를 확장하려면 틀 원문이 필요하다).

읽기(GET)만 한다. 편집·미리보기 등 쓰기 경로는 쓰지 않는다.
문서 본문은 CC BY-NC-SA 2.0 KR이므로 캐시는 저장소에 커밋하지 않는다(.gitignore).
"""

import base64
import pathlib
import re
import sys
import time
import urllib.parse
import urllib.request
import zlib

BASE_URL = "https://www.alphawiki.org"
CACHE_DIRECTORY = pathlib.Path(__file__).resolve().parent.parent / "target" / "parity-corpus"
USER_AGENT = "opensinabro-parity-harness (read-only parity comparison)"
REQUEST_INTERVAL_SECONDS = 0.5

# 파리티 대조에 쓸 기본 문서. the seed 문법을 폭넓게 쓰면서 동적 매크로가 적은 것들.
DEFAULT_DOCUMENTS = [
    "알파위키:문법 도움말",
    "알파위키:문법 도움말/심화",
]


def fetch_url(url):
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=90) as response:
        return response.read().decode("utf-8", "replace")


def initial_state(page_html):
    match = re.search(r'window\.INITIAL_STATE="([^"]+)"', page_html)
    if not match:
        return None
    return zlib.decompress(base64.b64decode(match.group(1)))


def read_varint(data, index):
    value = shift = 0
    while index < len(data):
        byte = data[index]
        index += 1
        value |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return value, index
        shift += 7
        if shift > 63:
            raise ValueError("varint too long")
    raise ValueError("truncated varint")


def protobuf_strings(data, out=None):
    """length-delimited 필드를 순서대로 순회하며 UTF-8 문자열을 수집한다."""
    if out is None:
        out = []
    index = 0
    while index < len(data):
        try:
            key, index = read_varint(data, index)
        except ValueError:
            return out
        wire_type = key & 7
        if wire_type == 2:
            try:
                length, index = read_varint(data, index)
            except ValueError:
                return out
            if length > len(data) - index:
                return out
            chunk = data[index : index + length]
            index += length
            try:
                out.append(chunk.decode("utf-8"))
            except UnicodeDecodeError:
                protobuf_strings(chunk, out)
        elif wire_type == 0:
            try:
                _, index = read_varint(data, index)
            except ValueError:
                return out
        elif wire_type == 5:
            index += 4
        elif wire_type == 1:
            index += 8
        else:
            return out
    return out


def fetch_source(document):
    """원문 마크업. state 안에서 마크업으로 보이는 가장 긴 문자열을 고른다."""
    state = initial_state(fetch_url(f"{BASE_URL}/raw/{urllib.parse.quote(document)}"))
    if state is None:
        return None
    candidates = protobuf_strings(state)
    markup = [text for text in candidates if "\n" in text or "[[" in text or "==" in text]
    return max(markup, key=len) if markup else None


def fetch_rendered(document):
    """렌더 본문. 문단별 HTML 조각을 필드 순서대로 이어 붙인다."""
    state = initial_state(fetch_url(f"{BASE_URL}/w/{urllib.parse.quote(document)}"))
    if state is None:
        return None
    fragments = [
        text
        for text in protobuf_strings(state)
        if text.lstrip().startswith("<") and "data-v-" in text
    ]
    return "\n".join(fragments) if fragments else None


def included_templates(source):
    return {
        name.strip()
        for name in re.findall(r"\[include\(([^,)]+)", source)
        if name.strip().startswith("틀:")
    }


def slug_of(document):
    return document.replace("/", "_").replace(":", "_")


def save(document, directory):
    slug = slug_of(document)
    source = fetch_source(document)
    time.sleep(REQUEST_INTERVAL_SECONDS)
    if source is None:
        print(f"  ! {document}: 원문 없음")
        return None
    (directory / f"{slug}.namu").write_text(source, encoding="utf-8")

    rendered = fetch_rendered(document)
    time.sleep(REQUEST_INTERVAL_SECONDS)
    if rendered is None:
        print(f"  ! {document}: 렌더 없음")
    else:
        (directory / f"{slug}.html").write_text(rendered, encoding="utf-8")
    print(f"  + {document} (원문 {len(source)}자, 렌더 {len(rendered or '')}자)")
    return source


def main():
    documents = sys.argv[1:] or DEFAULT_DOCUMENTS
    CACHE_DIRECTORY.mkdir(parents=True, exist_ok=True)
    print(f"캐시: {CACHE_DIRECTORY}")

    pending = list(documents)
    seen = set()
    while pending:
        document = pending.pop(0)
        if document in seen:
            continue
        seen.add(document)
        try:
            source = save(document, CACHE_DIRECTORY)
        except Exception as error:
            print(f"  ! {document}: {error}")
            continue
        if source is None:
            continue
        for template in included_templates(source) - seen:
            pending.append(template)

    print(f"완료: 문서 {len(seen)}건")


if __name__ == "__main__":
    main()
