use std::{net::SocketAddr, str::FromStr};

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use error::ApiError;
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use time::{SystemTimeService, TimeService};

mod db;
mod error;
mod time;

#[derive(Deserialize)]
struct RegisterRequest {
    nick: String,
    group: Option<String>,
    email: Option<String>,
    extra: Option<String>,
}

#[derive(sqlx::FromRow, Serialize)]
struct Visitor {
    nick: String,
    group: Option<String>,
}

#[derive(Clone)]
struct ApiState<T: TimeService> {
    time: T,
    db: SqlitePool,
}

fn api(time: impl TimeService, db: SqlitePool) -> Router {
    Router::new()
        .route("/register", post(add_visitor))
        .route("/visitors", get(list_visitors))
        .with_state(ApiState { time, db })
}

async fn add_visitor<T: TimeService>(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<ApiState<T>>,
    Json(request): Json<RegisterRequest>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        r#"INSERT INTO visitor (created_at, ip, nick, "group", email, extra) VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(state.time.now())
    .bind(
        headers
            .get("X-Forwarded-For")
            .map(|x| x.to_str().ok())
            .unwrap_or(Some(addr.to_string().as_str())),
    )
    .bind(request.nick)
    .bind(request.group)
    .bind(request.email)
    .bind(request.extra)
    .execute(&state.db)
    .await?;

    Ok(StatusCode::CREATED)
}

async fn list_visitors<T: TimeService>(
    State(state): State<ApiState<T>>,
) -> Result<(StatusCode, Json<Vec<Visitor>>), ApiError> {
    let visitors = sqlx::query_as::<_, Visitor>(r#"SELECT nick, "group" FROM visitor"#)
        .fetch_all(&state.db)
        .await?;

    Ok((StatusCode::OK, Json(visitors)))
}

#[tokio::main]
async fn main() {
    let db_options = SqliteConnectOptions::from_str("sqlite://data.db")
        .expect("bad connection string")
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    let db = SqlitePoolOptions::new()
        .connect_with(db_options)
        .await
        .expect("failed to open SQLite database");

    db::init(&db).await.expect("failed to initialize database");

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(api(SystemTimeService {}, db).into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr};

    use axum::http::Request;
    use hyper::Body;
    use tower::ServiceExt;

    use crate::time::ConstantTimeService;

    use super::*;

    async fn database() -> SqlitePool {
        let db = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::init(&db).await.unwrap();
        db
    }

    #[tokio::test]
    async fn can_register_using_only_nick() {
        let time = ConstantTimeService::new();
        let db = database().await;
        let api = api(time.clone(), db.clone());

        let response = api
            .oneshot(
                Request::builder()
                    .extension(ConnectInfo(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                        8080,
                    )))
                    .method("POST")
                    .uri("/register")
                    .header("Content-Type", "application/json")
                    .body(r#"{"nick":"Test"}"#.into())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        // Check created DB entry
        let visitor = sqlx::query_as::<_, db::Visitor>(r#"SELECT * FROM visitor"#)
            .fetch_one(&db)
            .await
            .unwrap();

        assert_eq!(visitor.id, 1);
        assert_eq!(visitor.created_at, time.now());
        assert_eq!(visitor.ip, "127.0.0.1:8080");
        assert_eq!(visitor.nick, "Test");
        assert_eq!(visitor.group, None);
        assert_eq!(visitor.email, None);
        assert_eq!(visitor.extra, None);
    }

    #[tokio::test]
    async fn can_register_with_all_fields() {
        let time = ConstantTimeService::new();
        let db = database().await;
        let api = api(time.clone(), db.clone());

        let response = api
            .oneshot(
                Request::builder()
                    .extension(ConnectInfo(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                        8080,
                    )))
                    .method("POST")
                    .uri("/register")
                    .header("Content-Type", "application/json")
                    .body(r#"{"nick":"Test","group":"Testerz","email":"test@example.com","extra":"Snacks"}"#.into())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        // Check created DB entry
        let visitor = sqlx::query_as::<_, db::Visitor>(r#"SELECT * FROM visitor"#)
            .fetch_one(&db)
            .await
            .unwrap();

        assert_eq!(visitor.id, 1);
        assert_eq!(visitor.created_at, time.now());
        assert_eq!(visitor.ip, "127.0.0.1:8080");
        assert_eq!(visitor.nick, "Test");
        assert_eq!(visitor.group.as_deref(), Some("Testerz"));
        assert_eq!(visitor.email.as_deref(), Some("test@example.com"));
        assert_eq!(visitor.extra.as_deref(), Some("Snacks"));
    }

    #[tokio::test]
    async fn can_list_visitors() {
        let time = ConstantTimeService::new();
        let db = database().await;
        let api = api(time.clone(), db.clone());

        insert_visitor(
            &db,
            Visitor {
                nick: "Groupless".into(),
                group: None,
            },
        )
        .await;

        insert_visitor(
            &db,
            Visitor {
                nick: "With Group".into(),
                group: Some("Awesome".into()),
            },
        )
        .await;

        let response = api
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/visitors")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            hyper::body::to_bytes(response.into_body())
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert_eq!(
            body,
            r#"[{"nick":"Groupless","group":null},{"nick":"With Group","group":"Awesome"}]"#
        );
    }

    async fn insert_visitor(db: &SqlitePool, visitor: Visitor) {
        sqlx::query(r#"INSERT INTO visitor (created_at, ip, nick, "group") VALUES (CURRENT_TIMESTAMP, '127.0.0.1:8080', $1, $2)"#)
            .bind(visitor.nick)
            .bind(visitor.group)
            .execute(db)
            .await
            .unwrap();
    }
}
