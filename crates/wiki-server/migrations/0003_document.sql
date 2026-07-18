-- wiki-document: 문서·리비전·역링크·파일·렌더 캐시 (docs/architecture.md)

CREATE TABLE document (
    id           BIGSERIAL PRIMARY KEY,
    namespace_id BIGINT NOT NULL REFERENCES namespace (id),
    title        TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL,
    UNIQUE (namespace_id, title)
);

CREATE TABLE revision (
    id            BIGSERIAL PRIMARY KEY,
    external_id   UUID NOT NULL UNIQUE,
    document_id   BIGINT NOT NULL REFERENCES document (id),
    sequence      BIGINT NOT NULL,
    kind_id       BIGINT NOT NULL REFERENCES revision_kind (id),
    actor_id      BIGINT NOT NULL REFERENCES actor (id),
    content       TEXT,
    comment       TEXT NOT NULL DEFAULT '',
    metadata      JSONB,
    content_bytes BIGINT NOT NULL,
    hidden        BOOLEAN NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL,
    UNIQUE (document_id, sequence)
);

CREATE INDEX revision_recent ON revision (created_at DESC);

CREATE TABLE document_reference (
    source_document_id  BIGINT NOT NULL REFERENCES document (id),
    target_namespace_id BIGINT NOT NULL REFERENCES namespace (id),
    target_title        TEXT NOT NULL,
    kind_id             BIGINT NOT NULL REFERENCES document_reference_kind (id),
    PRIMARY KEY (source_document_id, target_namespace_id, target_title, kind_id)
);

CREATE INDEX document_reference_target
    ON document_reference (target_namespace_id, target_title);

CREATE TABLE render_cache (
    document_id BIGINT PRIMARY KEY REFERENCES document (id),
    revision_id BIGINT NOT NULL REFERENCES revision (id),
    html        TEXT NOT NULL,
    rendered_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE star (
    user_id     BIGINT NOT NULL REFERENCES wiki_user (id),
    document_id BIGINT NOT NULL REFERENCES document (id),
    created_at  TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, document_id)
);

CREATE TABLE file_content (
    hash       TEXT PRIMARY KEY,
    media_type TEXT NOT NULL,
    byte_size  BIGINT NOT NULL,
    width      INTEGER,
    height     INTEGER,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE file_revision (
    revision_id  BIGINT PRIMARY KEY REFERENCES revision (id),
    content_hash TEXT NOT NULL REFERENCES file_content (hash),
    license_id   BIGINT NOT NULL REFERENCES license (id)
);
