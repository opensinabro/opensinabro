# 골든 테스트 픽스처

파서·렌더러 검증용 실제 나무위키 문서 원문입니다. 워크스페이스의 여러 크레이트가 공유합니다.

- 출처: 나무위키 데이터베이스 덤프 (2022-03-01), [heegyu/namuwiki](https://huggingface.co/datasets/heegyu/namuwiki) 경유
- 라이선스: 문서 본문은 [CC BY-NC-SA 2.0 KR](https://creativecommons.org/licenses/by-nc-sa/2.0/kr/) (나무위키 기여자 저작). 본 저장소의 MIT 라이선스는 코드에만 적용되며 이 디렉토리의 `.namu` 파일에는 적용되지 않습니다.
- 각 파일명은 문서 제목의 특수문자를 `_`로 치환한 것입니다 (`manifest.json` 참고).
- `corpus/` — 문법 하나만 담은 회귀 코퍼스. 실제 문서가 아니므로 아래 골든 대상에서는 제외됩니다. 케이스가 원문·의미론·렌더링을 한 파일에 담아 자기완결적이라 별도 골든 디렉토리가 없습니다. **원문 일부는 알파위키 문법 도움말의 예제를 그대로 옮긴 것입니다** — 출처와 라이선스는 `corpus/README.md` 참고.

실제 문서(`*.namu`)의 골든 스냅샷은 계층별로 나뉩니다.

| 계층 | 위치 |
|---|---|
| AST | `crates/namumark-parser/tests/golden-ast/` |
| 나무위키 동등 마크업 | `crates/namumark-backend-namuwiki/tests/golden-namuwiki/` |

갱신은 `UPDATE_GOLDEN=1`을 붙여 해당 테스트를 돌립니다. 단, 크레이트 전체나 워크스페이스에
한꺼번에 붙이면 **의도치 않은 골든까지 함께 덮어씁니다** — 바꾼 계층의 테스트만 지정하십시오.

```
UPDATE_GOLDEN=1 cargo test -p namumark-parser           --test golden_tests
UPDATE_GOLDEN=1 cargo test -p namumark-backend-namuwiki --test namuwiki_markup_golden_tests
UPDATE_GOLDEN=1 cargo test -p namumark-backend-namuwiki --test corpus_tests
```
