use std::env;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use tower::ServiceBuilder;

use crate::{db, error::ApiError, time::TimeService, ApiState};

pub fn routes<T: TimeService>() -> Router<ApiState<T>> {
    match env::var("API_KEY") {
        Err(_) => {
            eprintln!("API_KEY not set, /admin endpoints will be disabled");
            Router::new()
        }
        Ok(key) => Router::new()
            .route("/visitors", get(list_visitors))
            .route("/visitors/:id", delete(delete_visitor))
            .layer(
                ServiceBuilder::new()
                    .layer(tower_http::validate_request::ValidateRequestHeaderLayer::bearer(&key)),
            ),
    }
}

async fn list_visitors<T: TimeService>(
    State(state): State<ApiState<T>>,
) -> Result<(StatusCode, Json<Vec<db::Visitor>>), ApiError> {
    let visitors = sqlx::query_as::<_, db::Visitor>(r#"SELECT * FROM visitor ORDER BY id"#)
        .fetch_all(&state.db)
        .await?;

    Ok((StatusCode::OK, Json(visitors)))
}

async fn delete_visitor<T: TimeService>(
    Path(id): Path<i32>,
    State(state): State<ApiState<T>>,
) -> Result<StatusCode, ApiError> {
    let rows = sqlx::query(r#"DELETE FROM visitor WHERE id = ?"#)
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();

    match rows {
        0 => Ok(StatusCode::NOT_FOUND),
        _ => Ok(StatusCode::NO_CONTENT),
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use axum::body::Body;
    use http_body_util::BodyExt;
    use hyper::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::{
        testing,
        time::{ConstantTimeService, TimeService},
    };

    #[tokio::test]
    async fn should_require_key_to_list_visitors() {
        env::set_var("API_KEY", "key");

        let time = ConstantTimeService::new();
        let db = testing::database().await;
        let api = crate::api(time.clone(), db.clone());

        let response = api
            .oneshot(
                Request::builder()
                    .header("Authorization", "Bearer invalidkey")
                    .method("GET")
                    .uri("/admin/visitors")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn can_list_visitors() {
        env::set_var("API_KEY", "key");

        let time = ConstantTimeService::new();
        let db = testing::database().await;
        let api = crate::api(time.clone(), db.clone());

        testing::insert_visitor(&db, "Groupless", None).await;

        testing::insert_visitor(&db, "With Group", Some("Awesome".into())).await;

        let response = api
            .oneshot(
                Request::builder()
                    .header("Authorization", "Bearer key")
                    .method("GET")
                    .uri("/admin/visitors")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert_eq!(
            body,
            format!(
                r#"[{{"id":1,"created_at":"{0}","ip":"127.0.0.1:8080","nick":"Groupless","group":null,"email":null,"extra":null}},{{"id":2,"created_at":"{0}","ip":"127.0.0.1:8080","nick":"With Group","group":"Awesome","email":null,"extra":null}}]"#,
                time.now().format("%FT%TZ")
            )
        );
    }

    #[tokio::test]
    async fn should_require_key_to_delete_visitor() {
        env::set_var("API_KEY", "key");

        let time = ConstantTimeService::new();
        let db = testing::database().await;
        let api = crate::api(time.clone(), db.clone());

        let response = api
            .oneshot(
                Request::builder()
                    .header("Authorization", "Bearer invalidkey")
                    .method("DELETE")
                    .uri("/admin/visitors/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn can_delete_visitor() {
        env::set_var("API_KEY", "key");

        let time = ConstantTimeService::new();
        let db = testing::database().await;
        let api = crate::api(time.clone(), db.clone());

        testing::insert_visitor(&db, "Groupless", None).await;

        let response = api
            .oneshot(
                Request::builder()
                    .header("Authorization", "Bearer key")
                    .method("DELETE")
                    .uri("/admin/visitors/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let remaining: i32 = sqlx::query_scalar("SELECT COUNT(id) FROM visitor")
            .fetch_one(&db)
            .await
            .unwrap();

        assert_eq!(remaining, 0);
    }
}
