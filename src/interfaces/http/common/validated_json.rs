//! Validated JSON extractor for Axum
//!
//! `ValidatedJson<T>` works like `axum::Json<T>`, but additionally runs
//! `validator::Validate::validate()` on the deserialized value.
//! On validation failure it returns an automatic 422 response with
//! structured field-level error details.

use axum::extract::rejection::JsonRejection;
use axum::extract::FromRequest;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::de::DeserializeOwned;
use validator::Validate;

use super::ApiResponse;

/// An extractor that deserializes JSON and validates it.
///
/// # Usage
///
/// ```ignore
/// use validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct CreateUser {
///     #[validate(length(min = 1, max = 50))]
///     username: String,
///     #[validate(email)]
///     email: String,
/// }
///
/// async fn handler(ValidatedJson(body): ValidatedJson<CreateUser>) {
///     // `body` is guaranteed to pass validation
/// }
/// ```
pub struct ValidatedJson<T>(pub T);

/// Error type for `ValidatedJson` extraction failures.
pub enum ValidatedJsonRejection {
    /// JSON parsing failed.
    JsonError(JsonRejection),
    /// Validation failed.
    ValidationError(validator::ValidationErrors),
}

impl IntoResponse for ValidatedJsonRejection {
    fn into_response(self) -> Response {
        match self {
            Self::JsonError(rejection) => {
                let body = ApiResponse::<()>::error(format!("Invalid JSON: {}", rejection));
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            Self::ValidationError(errors) => {
                let field_errors: Vec<String> = errors
                    .field_errors()
                    .iter()
                    .flat_map(|(field, errs)| {
                        errs.iter().map(move |e| {
                            let msg = e
                                .message
                                .as_ref()
                                .map(|m| m.to_string())
                                .unwrap_or_else(|| format!("{:?}", e.code));
                            format!("{}: {}", field, msg)
                        })
                    })
                    .collect();

                let message = if field_errors.is_empty() {
                    "Validation failed".to_string()
                } else {
                    field_errors.join("; ")
                };

                let body = ApiResponse::<()>::error(message);
                (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
            }
        }
    }
}

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ValidatedJsonRejection;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(ValidatedJsonRejection::JsonError)?;

        value
            .validate()
            .map_err(ValidatedJsonRejection::ValidationError)?;

        Ok(ValidatedJson(value))
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use axum::Router;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct TestBody {
        #[validate(length(min = 1, max = 10))]
        name: String,
        #[validate(range(min = 1, max = 100))]
        age: u32,
    }

    async fn handler(ValidatedJson(_body): ValidatedJson<TestBody>) -> &'static str {
        "ok"
    }

    fn app() -> Router {
        Router::new().route("/test", post(handler))
    }

    async fn send(req: Request<Body>) -> axum::http::Response<Body> {
        use tower::Service;
        let mut svc = app().into_service();
        svc.call(req).await.unwrap()
    }

    #[tokio::test]
    async fn valid_body_returns_ok() {
        let body = serde_json::json!({"name": "Alice", "age": 30});
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = send(req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_json_returns_400() {
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from("not json"))
            .unwrap();

        let resp = send(req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn validation_failure_returns_422() {
        let body = serde_json::json!({"name": "", "age": 0});
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let resp = send(req).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
