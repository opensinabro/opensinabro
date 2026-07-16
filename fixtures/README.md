# 골든 테스트 픽스처

파서·렌더러 검증용 실제 나무위키 문서 원문입니다. 워크스페이스의 여러 크레이트가 공유합니다.

- 출처: 나무위키 데이터베이스 덤프 (2022-03-01), [heegyu/namuwiki](https://huggingface.co/datasets/heegyu/namuwiki) 경유
- 라이선스: 문서 본문은 [CC BY-NC-SA 2.0 KR](https://creativecommons.org/licenses/by-nc-sa/2.0/kr/) (나무위키 기여자 저작). 본 저장소의 MIT 라이선스는 코드에만 적용되며 이 디렉토리의 `.namu` 파일에는 적용되지 않습니다.
- 각 파일명은 문서 제목의 특수문자를 `_`로 치환한 것입니다 (`manifest.json` 참고).
- `corpus/` — 생성된 시험용 문서(kitchen sink 등). 실제 문서가 아니므로 골든 대상에서 제외됩니다.

골든 스냅샷 위치:
- AST: `crates/namumark-parser/tests/golden-ast/` — 갱신 `UPDATE_GOLDEN=1 cargo test -p namumark-parser --test golden_tests`
- 나무위키 동등 마크업: `crates/namumark-backend-namuwiki/tests/golden-namuwiki/` — 갱신 `UPDATE_GOLDEN=1 cargo test -p namumark-backend-namuwiki --test namuwiki_markup_golden_tests`
