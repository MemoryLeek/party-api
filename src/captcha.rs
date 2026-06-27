use hyper::Uri;
use serde::{Deserialize, Serialize};

pub use cap::Cap;

pub enum Error {
    InvalidToken,
    ServiceError(reqwest::Error),
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Error::ServiceError(value)
    }
}

pub trait CaptchaService: Clone + Send + Sync + 'static {
    fn verify(&self, token: &str) -> impl Future<Output = Result<(), Error>> + Send;
}

#[derive(Clone)]
pub struct NoOp {}

impl NoOp {
    pub fn new() -> Self {
        Self {}
    }
}

impl CaptchaService for NoOp {
    async fn verify(&self, _token: &str) -> Result<(), Error> {
        Ok(())
    }
}

mod cap {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(Serialize)]
    struct VerificationRequest<'a> {
        secret: &'a str,
        response: &'a str,
    }

    #[derive(Deserialize)]
    struct VerificationResponse {
        success: bool,
    }

    #[derive(Clone)]
    pub struct Cap {
        uri: Uri,
        secret: String,
    }

    impl Cap {
        pub fn new<S: Into<String>>(uri: Uri, secret: S) -> Self {
            Self {
                uri,
                secret: secret.into(),
            }
        }
    }

    impl CaptchaService for Cap {
        async fn verify(&self, token: &str) -> Result<(), Error> {
            reqwest::Client::new()
                .post(self.uri.to_string())
                .json(&VerificationRequest {
                    secret: &self.secret,
                    response: token,
                })
                .send()
                .await
                .map_err(Error::from)?
                .json::<VerificationResponse>()
                .await
                .map_err(Error::from)
                .and_then(|response| {
                    if response.success {
                        Ok(())
                    } else {
                        Err(Error::InvalidToken)
                    }
                })
        }
    }
}
