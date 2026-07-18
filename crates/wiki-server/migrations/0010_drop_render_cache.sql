-- 렌더 캐시 폐기: 무효화 근거가 1단계 역링크뿐이라 include 연쇄·리다이렉트에서
-- stale을 구조적으로 막을 수 없었다. 보기 요청은 매번 렌더한다.

DROP TABLE render_cache;
