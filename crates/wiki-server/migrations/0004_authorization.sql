-- wiki-authorization: ACL·aclgroup·perm (docs/architecture.md)
-- 상태를 지우지 않고 이력으로 쌓는다 — /block-history가 원본 행에서 나온다.

CREATE TABLE acl_rule (
    id                BIGSERIAL PRIMARY KEY,
    document_id       BIGINT REFERENCES document (id),
    namespace_id      BIGINT REFERENCES namespace (id),
    action_id         BIGINT NOT NULL REFERENCES acl_action (id),
    evaluation_order  BIGINT NOT NULL,
    condition_kind_id BIGINT NOT NULL REFERENCES acl_condition_kind (id),
    condition_value   TEXT NOT NULL,
    allowed           BOOLEAN NOT NULL,
    CHECK ((document_id IS NULL) <> (namespace_id IS NULL))
);

CREATE INDEX acl_rule_document ON acl_rule (document_id, action_id, evaluation_order);
CREATE INDEX acl_rule_namespace ON acl_rule (namespace_id, action_id, evaluation_order);

CREATE TABLE acl_group (
    id         BIGSERIAL PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE acl_group_member (
    id         BIGSERIAL PRIMARY KEY,
    group_id   BIGINT NOT NULL REFERENCES acl_group (id),
    actor_id   BIGINT REFERENCES actor (id),
    ip_range   TEXT,
    reason     TEXT NOT NULL,
    expires_at TIMESTAMPTZ,
    added_by   BIGINT NOT NULL REFERENCES actor (id),
    created_at TIMESTAMPTZ NOT NULL,
    removed_at TIMESTAMPTZ,
    removed_by BIGINT REFERENCES actor (id),
    CHECK ((actor_id IS NULL) <> (ip_range IS NULL))
);

CREATE INDEX acl_group_member_active
    ON acl_group_member (group_id, actor_id) WHERE removed_at IS NULL;

CREATE TABLE user_permission (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES wiki_user (id),
    permission_id BIGINT NOT NULL REFERENCES permission (id),
    granted_by    BIGINT NOT NULL REFERENCES actor (id),
    created_at    TIMESTAMPTZ NOT NULL,
    revoked_at    TIMESTAMPTZ,
    revoked_by    BIGINT REFERENCES actor (id)
);

-- 활성 부여는 하나 (회수 후 재부여하면 이력이 쌓인다).
CREATE UNIQUE INDEX user_permission_active
    ON user_permission (user_id, permission_id) WHERE revoked_at IS NULL;
