use super::*;
use axum::http::Request;

async fn parse(value: Option<&str>) -> Result<IfMatch, ApiError> {
    let mut builder = Request::builder().uri("/");
    if let Some(value) = value {
        builder = builder.header(IF_MATCH, value);
    }
    let (mut parts, _) = builder.body(()).unwrap().into_parts();
    IfMatch::from_request_parts(&mut parts, &()).await
}

#[tokio::test]
async fn accepts_a_strong_etag() {
    assert_eq!(parse(Some("\"7\"")).await.unwrap().0, Revision::new(7));
}

#[tokio::test]
async fn accepts_a_weak_etag() {
    assert_eq!(parse(Some("W/\"7\"")).await.unwrap().0, Revision::new(7));
}

#[tokio::test]
async fn accepts_a_bare_number() {
    assert_eq!(parse(Some("7")).await.unwrap().0, Revision::new(7));
}

#[tokio::test]
async fn a_missing_header_is_precondition_required() {
    let err = parse(None).await.unwrap_err();
    assert_eq!(err.status(), StatusCode::PRECONDITION_REQUIRED);
}

#[tokio::test]
async fn a_wildcard_is_refused() {
    let err = parse(Some("*")).await.unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn nonsense_is_refused() {
    let err = parse(Some("\"banana\"")).await.unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
}
