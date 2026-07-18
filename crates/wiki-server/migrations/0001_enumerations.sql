-- 닫힌 값 집합은 열거 테이블로 둔다 (docs/design/08). 코드는 name으로 참조하고,
-- 값 추가는 DDL이 아니라 이 시드에 행을 더하는 일이 된다.

CREATE TABLE namespace (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE revision_kind (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE document_reference_kind (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE license (
    id           BIGSERIAL PRIMARY KEY,
    name         TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    source_url   TEXT
);

CREATE TABLE credential_kind (
    id              BIGSERIAL PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    allows_multiple BOOLEAN NOT NULL
);

CREATE TABLE verification_purpose (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE notification_kind (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE permission (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE acl_action (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE acl_condition_kind (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE thread_status (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE thread_comment_kind (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE edit_request_status (
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

INSERT INTO namespace (name) VALUES
    ('문서'), ('틀'), ('분류'), ('파일'), ('사용자'), ('위키운영'), ('휴지통');

INSERT INTO revision_kind (name) VALUES
    ('create'), ('edit'), ('move'), ('delete'), ('restore'), ('revert'), ('import');

INSERT INTO document_reference_kind (name) VALUES
    ('link'), ('include'), ('redirect'), ('image'), ('category');

INSERT INTO credential_kind (name, allows_multiple) VALUES
    ('password', false), ('totp', false), ('passkey', true), ('oauth', true), ('email', true);

INSERT INTO verification_purpose (name) VALUES
    ('signup'), ('password_reset'), ('email_change'), ('device');

INSERT INTO notification_kind (name) VALUES
    ('thread_comment'), ('edit_request_reviewed');

INSERT INTO permission (name) VALUES
    ('admin'), ('grant'), ('aclgroup'), ('nsacl'), ('delete_thread'),
    ('hide_thread_comment'), ('update_thread_status'), ('update_thread_document'),
    ('update_thread_topic'), ('hide_revision'), ('batch_revert'), ('login_history'),
    ('config'), ('api_access'), ('skip_captcha'), ('developer');

INSERT INTO acl_action (name) VALUES
    ('read'), ('edit'), ('move'), ('delete'), ('create_thread'),
    ('write_thread_comment'), ('edit_request'), ('acl');

INSERT INTO acl_condition_kind (name) VALUES
    ('perm'), ('user'), ('ip'), ('geoip'), ('aclgroup');

INSERT INTO thread_status (name) VALUES
    ('normal'), ('pause'), ('close');

INSERT INTO thread_comment_kind (name) VALUES
    ('comment'), ('status_change'), ('topic_change'), ('document_move');

INSERT INTO edit_request_status (name) VALUES
    ('open'), ('accepted'), ('closed');
