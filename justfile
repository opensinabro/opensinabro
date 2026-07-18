# opensinabro 개발 작업 모음.
#
# 데이터베이스는 도커 컴포즈가 띄우고, 서버는 로컬에서 카고로 돌린다:
#   just setup   # 데이터베이스 컨테이너 준비 (처음 한 번)
#   just import  # 문서 적재
#   just run     # 서버
#
# 서버까지 컨테이너로 돌리려면 `just docker-up`.

database_name := env("OPENSINABRO_DATABASE", "opensinabro")
database_password := env("POSTGRES_PASSWORD", "opensinabro")
database_port := env("OPENSINABRO_DATABASE_PORT", "5432")
database_url := env(
    "DATABASE_URL",
    "postgres://opensinabro:" + database_password + "@localhost:" + database_port + "/" + database_name
)
data_directory := env("OPENSINABRO_DATA", "data")
address := env("OPENSINABRO_ADDRESS", "127.0.0.1:3000")

# 무엇을 할 수 있는지 보인다.
default:
    @just --list

# 로컬에서 처음 한 번: 데이터베이스 컨테이너를 띄운다.
# 데이터베이스와 스키마는 각각 컴포즈와 서버가 알아서 만든다.
setup: database-up
    @echo "준비됐습니다. just run 으로 서버를 띄우세요."

# 데이터베이스 컨테이너를 띄우고 접속을 받을 때까지 기다린다.
database-up:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose up --detach --wait database
    echo "PostgreSQL이 localhost:{{ database_port }}에서 기다립니다."

database-down:
    docker compose stop database

# 데이터베이스 셸.
database-shell:
    docker compose exec database psql --username=opensinabro --dbname={{ database_name }}

# 데이터베이스를 통째로 비운다. 검색 색인과 올린 파일도 함께 지운다.
database-reset:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose down --volumes
    rm -rf "{{ data_directory }}"
    just database-up
    echo "데이터베이스와 로컬 데이터를 비웠습니다."

# 서버를 띄운다.
run: database-up
    DATABASE_URL="{{ database_url }}" \
    OPENSINABRO_DATA="{{ data_directory }}" \
    OPENSINABRO_ADDRESS="{{ address }}" \
    cargo run --release --package wiki-server --bin opensinabro

# 서버를 개발 모드로 띄운다 (빌드가 빠르고 실행이 느리다).
dev: database-up
    DATABASE_URL="{{ database_url }}" \
    OPENSINABRO_DATA="{{ data_directory }}" \
    OPENSINABRO_ADDRESS="{{ address }}" \
    cargo run --package wiki-server --bin opensinabro

# 나무마크 원문을 적재한다. 색인은 서버가 다음에 시작할 때 만든다.
import path="fixtures/documents": database-up
    DATABASE_URL="{{ database_url }}" \
    OPENSINABRO_DATA="{{ data_directory }}" \
    cargo run --release --package wiki-server --bin opensinabro-import -- "{{ path }}"

# 처음 쓰는 사람을 위해: 준비 → 적재 → 실행을 잇는다.
start: setup import run

# 서버까지 컨테이너로 띄운다.
docker-up:
    docker compose up --detach

docker-down:
    docker compose down

# 컨테이너 서버에 원문을 적재한다.
docker-import path="fixtures/documents":
    docker compose run --rm --volume "$(pwd)/{{ path }}:/import:ro" \
        wiki opensinabro-import /import
    docker compose restart wiki

# 모든 컨테이너를 내린다 (데이터는 남는다).
down:
    docker compose down

test:
    cargo test --workspace

lint:
    cargo clippy --workspace --all-targets

format:
    cargo fmt --all

# 커밋 전에 한 번 돌리는 것들.
check: format lint test
