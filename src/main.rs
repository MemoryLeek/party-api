use std::{env, net::SocketAddr, str::FromStr};

use axum::{
    extract::{ConnectInfo, State},
    handler::Handler,
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
use tokio::signal;
use tower::ServiceBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};

mod admin;
mod cors;
mod db;
mod error;
#[cfg(test)]
mod testing;
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
    id: i32,
    nick: String,
    group: Option<String>,
}

#[derive(Clone)]
pub struct ApiState<T: TimeService> {
    time: T,
    db: SqlitePool,
}

fn api(time: impl TimeService, db: SqlitePool) -> Router {
    let add_visitor_rate_config = Box::new(
        GovernorConfigBuilder::default()
            .per_second(60)
            .burst_size(3)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );

    let add_visitor_rate_limit = ServiceBuilder::new()
        .layer(axum::error_handling::HandleErrorLayer::new(
            |e: axum::BoxError| async move { ApiError::from(e) },
        ))
        .layer(GovernorLayer {
            config: Box::leak(add_visitor_rate_config),
        });

    Router::new()
        .route("/register", post(add_visitor.layer(add_visitor_rate_limit)))
        .route("/visitors", get(list_visitors))
        .nest("/admin", admin::routes())
        .layer(cors::layer())
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
    let visitors = sqlx::query_as::<_, Visitor>(r#"SELECT id, nick, "group" FROM visitor"#)
        .fetch_all(&state.db)
        .await?;

    Ok((StatusCode::OK, Json(visitors)))
}

#[tokio::main]
async fn main() {
    let db_connection_string = format!(
        "sqlite://{}",
        env::var("SQLITE_DB").unwrap_or("data.db".into())
    );
    let db_options = SqliteConnectOptions::from_str(&db_connection_string)
        .expect(&format!("bad connection string: {}", db_connection_string))
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    let db = SqlitePoolOptions::new()
        .connect_with(db_options)
        .await
        .expect("failed to open SQLite database");

    db::init(&db).await.expect("failed to initialize database");

    let addr = env::var("LISTEN_ADDR").unwrap_or("127.0.0.1:3000".into());
    let socket_address = SocketAddr::from_str(&addr).expect(&format!("bad LISTEN_ADDR: {}", addr));

    axum::Server::bind(&socket_address)
        .serve(api(SystemTimeService {}, db).into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr};

    use axum::{http::Request, response::IntoResponse};
    use hyper::Body;
    use tower::{Service, ServiceExt};

    use crate::time::ConstantTimeService;

    use super::*;

    #[tokio::test]
    async fn can_register_using_only_nick() {
        let time = ConstantTimeService::new();
        let db = testing::database().await;
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
    async fn can_only_register_single_nick() {
        let time = ConstantTimeService::new();
        let db = testing::database().await;
        let api = api(time.clone(), db.clone());

        testing::insert_visitor(&db, "Only One Nick", None).await;

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
                    .body(r#"{"nick":"Only One Nick"}"#.into())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = String::from_utf8(
            hyper::body::to_bytes(response.into_body())
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert_eq!(
            body,
            r#"{"error":"error returned from database: (code: 2067) UNIQUE constraint failed: visitor.nick"}"#
        );
    }

    #[tokio::test]
    async fn can_register_with_all_fields() {
        let time = ConstantTimeService::new();
        let db = testing::database().await;
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
    async fn should_rate_limit_register() {
        let time = ConstantTimeService::new();
        let db = testing::database().await;
        let mut api = api(time.clone(), db.clone());

        async fn register(api: &mut Router, nick: &str) -> impl IntoResponse {
            api.ready()
                .await
                .unwrap()
                .call(
                    Request::builder()
                        .extension(ConnectInfo(SocketAddr::new(
                            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                            8080,
                        )))
                        .method("POST")
                        .uri("/register")
                        .header("Content-Type", "application/json")
                        .body(format!(r#"{{"nick":"{}"}}"#, nick).into())
                        .unwrap(),
                )
                .await
                .unwrap()
        }

        let response = register(&mut api, "One").await.into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        let response = register(&mut api, "Two").await.into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        let response = register(&mut api, "Three").await.into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        let response = register(&mut api, "Four should fail").await.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn can_list_visitors() {
        let time = ConstantTimeService::new();
        let db = testing::database().await;
        let api = api(time.clone(), db.clone());

        testing::insert_visitor(&db, "Groupless", None).await;

        testing::insert_visitor(&db, "With Group", Some("Awesome".into())).await;

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
            r#"[{"id":1,"nick":"Groupless","group":null},{"id":2,"nick":"With Group","group":"Awesome"}]"#
        );
    }
}
