# opensinabro 개발 작업 모음.
#
# 로컬에서 위키를 띄우는 명령은 하나다:
#   just dev
#
# 데이터베이스 준비·의존성 설치·포트 정리·예시 문서 적재까지 알아서 하므로,
# 처음 받은 사람도 이것만 치면 된다. Ctrl+C 한 번으로 전부 멈춘다.
#
# 브라우저는 언제나 백엔드 주소 하나(http://127.0.0.1:3000)로 들어간다. 프론트엔드는
# 3001에서 따로 돌지만 백엔드가 대신 넘겨주므로 직접 열 일이 없다 (docs/design/07).
#
# 전부 컨테이너로 돌리려면 `just docker-up`.

database_name := env("OPENSINABRO_DATABASE", "opensinabro")
database_password := env("POSTGRES_PASSWORD", "opensinabro")
database_port := env("OPENSINABRO_DATABASE_PORT", "5432")
database_url := env(
    "DATABASE_URL",
    "postgres://opensinabro:" + database_password + "@localhost:" + database_port + "/" + database_name
)
data_directory := env("OPENSINABRO_DATA", "data")
backend_port := env("OPENSINABRO_PORT", "3000")
address := env("OPENSINABRO_ADDRESS", "127.0.0.1:" + backend_port)
frontend_port := env("OPENSINABRO_FRONTEND_PORT", "3001")
frontend_origin := env("OPENSINABRO_FRONTEND", "http://127.0.0.1:" + frontend_port)

# 무엇을 할 수 있는지 보인다.
[private]
default:
    @just --list --list-heading $'\n무엇을 할 수 있는지:\n'

# ── 실행 ──────────────────────────────────────────────────────────────────

# 위키를 띄운다. 준비가 안 됐으면 알아서 갖춘다.
[group('실행')]
dev: _ready free-ports
    #!/usr/bin/env bash
    set -euo pipefail

    # 프론트엔드를 뒤에서 돌리므로 이 셸이 끝날 때 함께 정리한다. `kill 0`은 프로세스
    # 그룹 전체를 때려 나란히 돌던 다른 작업까지 죽이므로 쓰지 않는다 — 작업 제어를
    # 켜서 프론트엔드에 자기 그룹을 주고, 그 그룹만 종료한다(npm이 띄운 next까지).
    set -m
    (cd frontend && npm run dev -- --port {{ frontend_port }}) &
    frontend=$!
    trap 'kill -- -"$frontend" 2> /dev/null || true' EXIT
    set +m

    echo "브라우저에서 http://{{ address }} 를 여세요."
    DATABASE_URL="{{ database_url }}" \
    OPENSINABRO_DATA="{{ data_directory }}" \
    OPENSINABRO_ADDRESS="{{ address }}" \
    OPENSINABRO_FRONTEND="{{ frontend_origin }}" \
    cargo run --package wiki-server --bin opensinabro

# 백엔드만. 프론트엔드를 다른 터미널에서 돌릴 때 쓴다.
[group('실행')]
back: _database (free-port backend_port)
    DATABASE_URL="{{ database_url }}" \
    OPENSINABRO_DATA="{{ data_directory }}" \
    OPENSINABRO_ADDRESS="{{ address }}" \
    OPENSINABRO_FRONTEND="{{ frontend_origin }}" \
    cargo run --package wiki-server --bin opensinabro

# 프론트엔드만.
[group('실행')]
front: _dependencies (free-port frontend_port)
    cd frontend && npm run dev -- --port {{ frontend_port }}

# 3000·3001을 물고 있는 프로세스를 정리한다. 띄우기 전에 자동으로 돈다.
[group('실행')]
free-ports: (free-port backend_port) (free-port frontend_port)

# ── 문서 ──────────────────────────────────────────────────────────────────

# 같은 원문을 두 번 넣으면 리비전이 그만큼 쌓인다. 처음부터 다시 하려면
# `just database-reset`을 먼저 돌린다.
#
# 나무마크 원문을 적재한다. 색인은 서버가 다음에 시작할 때 만든다.
[group('문서')]
import path="fixtures/documents": _database
    DATABASE_URL="{{ database_url }}" \
    OPENSINABRO_DATA="{{ data_directory }}" \
    cargo run --release --package wiki-server --bin opensinabro-import -- "{{ path }}"

# ── 데이터베이스 ───────────────────────────────────────────────────────────

# 데이터베이스 셸.
[group('데이터베이스')]
database-shell:
    docker compose exec database psql --username=opensinabro --dbname={{ database_name }}

# 데이터베이스를 통째로 비운다. 검색 색인과 올린 파일도 함께 지운다.
[group('데이터베이스')]
database-reset:
    #!/usr/bin/env bash
    set -euo pipefail
    docker compose down --volumes
    rm -rf "{{ data_directory }}"
    echo "데이터베이스와 로컬 데이터를 비웠습니다. just dev 로 다시 띄우세요."

# ── 컨테이너 ───────────────────────────────────────────────────────────────

# 위키까지 전부 컨테이너로 띄운다.
[group('컨테이너')]
docker-up:
    docker compose up --detach

# 모든 컨테이너를 내린다 (데이터는 남는다).
[group('컨테이너')]
docker-down:
    docker compose down

# 컨테이너 위키에 원문을 적재한다.
[group('컨테이너')]
docker-import path="fixtures/documents":
    docker compose run --rm --volume "$(pwd)/{{ path }}:/import:ro" \
        wiki opensinabro-import /import
    docker compose restart wiki

# ── 점검 ──────────────────────────────────────────────────────────────────

[group('점검')]
test:
    cargo test --workspace

[group('점검')]
lint:
    cargo clippy --workspace --all-targets

[group('점검')]
format:
    cargo fmt --all

# 커밋 전에 한 번 돌리는 것들.
[group('점검')]
check: format lint test

# ── 여기부터는 다른 명령이 알아서 부르는 것들 ────────────────────────────────

# 띄울 준비 — 데이터베이스·의존성·첫 문서.
[private]
_ready: _database _dependencies _seed

# 데이터베이스 컨테이너를 띄우고 접속을 받을 때까지 기다린다.
# 스키마는 서버와 임포터가 시작할 때 만든다.
[private]
_database:
    @docker compose up --detach --wait database

# 프론트엔드 의존성. 이미 있으면 건너뛴다.
[private]
_dependencies:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -d frontend/node_modules ]; then
        echo "프론트엔드 의존성을 설치합니다..."
        cd frontend && npm install
    fi

# 위키가 비어 있을 때만 예시 문서를 넣는다.
#
# 문서가 이미 있으면 건드리지 않는다 — 띄울 때마다 적재하면 같은 원문이 리비전으로
# 쌓여 역사가 거짓이 된다.
[private]
_seed: _database
    #!/usr/bin/env bash
    set -euo pipefail
    documents=$(docker compose exec -T database \
        psql --username=opensinabro --dbname={{ database_name }} \
        --tuples-only --no-align --command "SELECT count(*) FROM document" 2> /dev/null || echo 0)

    if [ "${documents//[!0-9]/}" = "0" ]; then
        echo "위키가 비어 있어 예시 문서를 적재합니다..."
        just import
    fi

# 이 포트를 물고 있는 프로세스를 정리한다.
#
# 앞서 띄운 서버가 제대로 안 죽으면 기동이 "주소가 이미 쓰이는 중"으로 실패한다.
[private]
free-port port:
    #!/usr/bin/env bash
    set -euo pipefail

    if ! command -v lsof > /dev/null; then
        echo "lsof가 없어 {{ port }} 포트를 확인하지 못했습니다. 떠 있는 서버가 있으면 직접 멈추세요."
        exit 0
    fi

    listeners=$(lsof -ti "tcp:{{ port }}" -sTCP:LISTEN || true)
    if [ -z "$listeners" ]; then
        exit 0
    fi

    echo "{{ port }} 포트를 쓰던 프로세스를 정리합니다 (PID: ${listeners})."
    # shellcheck disable=SC2086
    kill $listeners 2> /dev/null || true

    for _ in 1 2 3 4 5; do
        sleep 1
        if [ -z "$(lsof -ti "tcp:{{ port }}" -sTCP:LISTEN || true)" ]; then
            exit 0
        fi
    done

    remaining=$(lsof -ti "tcp:{{ port }}" -sTCP:LISTEN || true)
    if [ -n "$remaining" ]; then
        echo "{{ port }} 포트가 아직 잡혀 있어 강제로 종료합니다 (PID: ${remaining})."
        # shellcheck disable=SC2086
        kill -9 $remaining 2> /dev/null || true
    fi
