-- wiki-account: 행위 주체와 인증 (docs/design/08)
-- 이메일은 별도 테이블이 아니라 user_credential의 한 kind다.

CREATE TABLE wiki_user (
    id          BIGSERIAL PRIMARY KEY,
    external_id UUID NOT NULL UNIQUE,
    name        TEXT NOT NULL UNIQUE,
    is_system   BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL
);

CREATE TABLE actor (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT REFERENCES wiki_user (id),
    ip_address TEXT,
    CHECK ((user_id IS NULL) <> (ip_address IS NULL))
);

CREATE UNIQUE INDEX actor_user ON actor (user_id) WHERE user_id IS NOT NULL;
CREATE UNIQUE INDEX actor_ip ON actor (ip_address) WHERE ip_address IS NOT NULL;

CREATE TABLE user_credential (
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES wiki_user (id),
    kind_id      BIGINT NOT NULL REFERENCES credential_kind (id),
    label        TEXT,
    identifier   TEXT,
    secret       TEXT,
    verified_at  TIMESTAMPTZ,
    is_primary   BOOLEAN NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ
);

-- 같은 주소·보안키·외부 계정이 두 사용자에 붙지 않게.
CREATE UNIQUE INDEX user_credential_identifier
    ON user_credential (kind_id, identifier) WHERE identifier IS NOT NULL;
-- kind별 주 수단은 하나.
CREATE UNIQUE INDEX user_credential_primary
    ON user_credential (user_id, kind_id) WHERE is_primary;

CREATE TABLE user_verification (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES wiki_user (id),
    credential_id BIGINT REFERENCES user_credential (id),
    purpose_id    BIGINT NOT NULL REFERENCES verification_purpose (id),
    token_hash    TEXT NOT NULL UNIQUE,
    expires_at    TIMESTAMPTZ NOT NULL,
    consumed_at   TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL
);

CREATE TABLE login_record (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES wiki_user (id),
    credential_id BIGINT REFERENCES user_credential (id),
    ip_address    TEXT NOT NULL,
    user_agent    TEXT NOT NULL,
    succeeded     BOOLEAN NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL
);

CREATE INDEX login_record_user ON login_record (user_id, created_at);

CREATE TABLE user_preference (
    user_id BIGINT NOT NULL REFERENCES wiki_user (id),
    name    TEXT NOT NULL,
    data    TEXT NOT NULL,
    PRIMARY KEY (user_id, name)
);

CREATE TABLE notification (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES wiki_user (id),
    kind_id    BIGINT NOT NULL REFERENCES notification_kind (id),
    payload    JSONB NOT NULL,
    read_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL
);
