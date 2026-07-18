-- 세션은 wiki-server가 소유한다 (docs/design/08).
-- 값은 세션 계층이 직렬화한 바이트라 여기서는 열지 않는다.

CREATE TABLE session (
    id         TEXT PRIMARY KEY,
    data       BYTEA NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX session_expiry ON session (expires_at);
