-- wiki-server: 전역 설정 (docs/architecture.md). 세션 테이블은 세션 크레이트가 관리한다.

CREATE TABLE site_setting (
    name TEXT PRIMARY KEY,
    data TEXT NOT NULL
);
