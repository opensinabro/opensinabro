# 골든 테스트 픽스처

파서 검증용 실제 나무위키 문서 원문입니다.

- 출처: 나무위키 데이터베이스 덤프 (2022-03-01), [heegyu/namuwiki](https://huggingface.co/datasets/heegyu/namuwiki) 경유
- 라이선스: 문서 본문은 [CC BY-NC-SA 2.0 KR](https://creativecommons.org/licenses/by-nc-sa/2.0/kr/) (나무위키 기여자 저작). 본 저장소의 MIT 라이선스는 코드에만 적용되며 이 디렉토리의 `.namu` 파일에는 적용되지 않습니다.
- 각 파일명은 문서 제목의 특수문자를 `_`로 치환한 것입니다 (`manifest.json` 참고).

`golden/` 디렉토리의 `.ast` 파일은 각 문서의 기대 파싱 결과 스냅샷입니다.
갱신: `UPDATE_GOLDEN=1 cargo test --test golden_tests`
