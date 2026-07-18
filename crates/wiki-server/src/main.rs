//! 위키 서버 실행 바이너리.

use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let data_directory = std::env::var("OPENSINABRO_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"));
    let address =
        std::env::var("OPENSINABRO_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL을 설정하세요 (예: postgres://opensinabro@localhost/opensinabro).");
        std::process::exit(2);
    };

    if let Err(error) = std::fs::create_dir_all(&data_directory) {
        eprintln!("데이터 디렉터리를 만들지 못했습니다: {error}");
        std::process::exit(1);
    }

    let state = match wiki_server::open_state(
        &database_url,
        &data_directory.join("search-index"),
        &data_directory.join("files"),
    )
    .await
    {
        Ok(state) => state,
        Err(error) => {
            eprintln!("저장소를 열지 못했습니다: {error}");
            std::process::exit(1);
        }
    };

    let listener = match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("{address}에서 수신할 수 없습니다: {error}");
            std::process::exit(1);
        }
    };

    println!("opensinabro가 http://{address} 에서 실행 중입니다.");

    // 비로그인 편집자는 IP가 곧 신원이라(actor) 연결 정보를 핸들러에 넘긴다.
    let service =
        wiki_server::router(state).into_make_service_with_connect_info::<std::net::SocketAddr>();

    if let Err(error) = axum::serve(listener, service).await {
        eprintln!("서버가 멈췄습니다: {error}");
        std::process::exit(1);
    }
}
