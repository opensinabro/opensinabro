# 빌드와 실행을 나눠 최종 이미지에는 바이너리와 실행에 필요한 것만 남긴다.
FROM rust:1.96-slim-bookworm AS builder

WORKDIR /build

RUN apt-get update \
    && apt-get install --yes --no-install-recommends pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY tools tools

# 위키 서버만 만든다 — 파서·렌더러는 의존성으로 함께 빌드된다.
RUN cargo build --release --package wiki-server

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 opensinabro

COPY --from=builder /build/target/release/opensinabro /usr/local/bin/opensinabro
COPY --from=builder /build/target/release/opensinabro-import /usr/local/bin/opensinabro-import

# 검색 색인은 DB 밖 파일이라 볼륨으로 남긴다.
ENV OPENSINABRO_DATA=/var/lib/opensinabro \
    OPENSINABRO_ADDRESS=0.0.0.0:3000
RUN mkdir -p /var/lib/opensinabro && chown opensinabro:opensinabro /var/lib/opensinabro

USER opensinabro
EXPOSE 3000

CMD ["opensinabro"]
