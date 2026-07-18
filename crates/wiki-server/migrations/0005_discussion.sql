-- wiki-discussion: 토론·편집요청 (docs/architecture.md)

CREATE TABLE thread (
    id          BIGSERIAL PRIMARY KEY,
    external_id UUID NOT NULL UNIQUE,
    document_id BIGINT NOT NULL REFERENCES document (id),
    topic       TEXT NOT NULL,
    status_id   BIGINT NOT NULL REFERENCES thread_status (id),
    created_at  TIMESTAMPTZ NOT NULL
);

CREATE INDEX thread_document ON thread (document_id);

CREATE TABLE thread_comment (
    id           BIGSERIAL PRIMARY KEY,
    thread_id    BIGINT NOT NULL REFERENCES thread (id),
    sequence     BIGINT NOT NULL,
    kind_id      BIGINT NOT NULL REFERENCES thread_comment_kind (id),
    actor_id     BIGINT NOT NULL REFERENCES actor (id),
    content      TEXT NOT NULL DEFAULT '',
    metadata     JSONB,
    admin_marked BOOLEAN NOT NULL DEFAULT false,
    hidden_at    TIMESTAMPTZ,
    hidden_by    BIGINT REFERENCES actor (id),
    created_at   TIMESTAMPTZ NOT NULL,
    UNIQUE (thread_id, sequence)
);

CREATE TABLE edit_request (
    id               BIGSERIAL PRIMARY KEY,
    external_id      UUID NOT NULL UNIQUE,
    document_id      BIGINT NOT NULL REFERENCES document (id),
    base_revision_id BIGINT REFERENCES revision (id),
    actor_id         BIGINT NOT NULL REFERENCES actor (id),
    content          TEXT NOT NULL,
    comment          TEXT NOT NULL DEFAULT '',
    status_id        BIGINT NOT NULL REFERENCES edit_request_status (id),
    reviewed_by      BIGINT REFERENCES actor (id),
    created_at       TIMESTAMPTZ NOT NULL
);

CREATE INDEX edit_request_document ON edit_request (document_id);
