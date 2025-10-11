use std::env;

use axum::http::{HeaderValue, Method};
use tower::{
    ServiceBuilder,
    layer::util::{Identity, Stack},
};
use tower_http::cors::{self, CorsLayer};

pub fn layer() -> ServiceBuilder<Stack<CorsLayer, Identity>> {
    let origin = if let Ok(value) = env::var("CORS_ORIGIN") {
        HeaderValue::from_str(&value).expect("failed to parse CORS_ORIGIN value")
    } else {
        HeaderValue::from_static("*")
    };

    let cors = CorsLayer::new()
        .allow_headers(cors::Any)
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_origin(origin);

    ServiceBuilder::new().layer(cors)
}

#[cfg(test)]
mod test {
    use std::env;

    use axum::body::Body;
    use hyper::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::{pow::Sha256ProofOfWorkService, testing, time::ConstantTimeService};

    #[tokio::test]
    async fn should_allow_any_by_default() {
        let time = ConstantTimeService::new();
        let pow = Sha256ProofOfWorkService::new(0);
        let db = testing::database().await;
        let api = crate::api(time.clone(), pow, db.clone());

        let response = api
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/register")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            Some("*"),
            response
                .headers()
                .get("Access-Control-Allow-Origin")
                .map(|x| x.to_str().unwrap())
        );
        assert_eq!(
            Some("GET,POST"),
            response
                .headers()
                .get("Access-Control-Allow-Methods")
                .map(|x| x.to_str().unwrap())
        );
        assert_eq!(
            Some("*"),
            response
                .headers()
                .get("Access-Control-Allow-Headers")
                .map(|x| x.to_str().unwrap())
        );
    }

    #[tokio::test]
    async fn should_allow_override_by_env() {
        unsafe { env::set_var("CORS_ORIGIN", "http://example.com") };

        let time = ConstantTimeService::new();
        let pow = Sha256ProofOfWorkService::new(0);
        let db = testing::database().await;
        let api = crate::api(time.clone(), pow, db.clone());

        let response = api
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/register")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            Some("http://example.com"),
            response
                .headers()
                .get("Access-Control-Allow-Origin")
                .map(|x| x.to_str().unwrap())
        );
        assert_eq!(
            Some("GET,POST"),
            response
                .headers()
                .get("Access-Control-Allow-Methods")
                .map(|x| x.to_str().unwrap())
        );
    }
}
