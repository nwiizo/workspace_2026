use serde::Serialize;
use tatami_log::{db, web};
use topcoat::router::{Body, Request, Response, Router, StatusCode, header, to_bytes};

async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    form: Option<String>,
    cookie: Option<&str>,
) -> Response {
    let mut request = Request::builder().method(method).uri(uri);
    if form.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    }
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    router
        .handle(
            request
                .body(form.map_or_else(Body::empty, Body::from))
                .expect("request should be valid"),
        )
        .await
}

async fn body_text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    String::from_utf8(bytes.to_vec()).expect("response should be UTF-8")
}

fn session_cookie(response: &Response) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .expect("session response should set a cookie")
        .to_str()
        .expect("cookie should be ASCII")
        .split(';')
        .next()
        .expect("cookie should contain a name and value")
        .to_owned()
}

#[derive(Serialize)]
struct Credentials<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
struct Training<'a> {
    trained_on: &'a str,
    uniform: &'a str,
    rounds: u64,
    reflection: &'a str,
    name: &'a str,
    position: &'a str,
    cue: &'a str,
    answer: &'a str,
    next_try: &'a str,
}

async fn register(router: &Router, email: &str) -> (Response, String) {
    let form = serde_urlencoded::to_string(Credentials {
        email,
        password: "十分に長いパスワードです",
    })
    .expect("credentials should serialize");
    let response = send(router, "POST", "/register", Some(form), None).await;
    let cookie = session_cookie(&response);
    (response, cookie)
}

#[tokio::test]
async fn unauthenticated_app_requests_redirect_to_login() {
    let database = db::connect("sqlite::memory:")
        .await
        .expect("in-memory database should open");
    let router = web::router(database, None);
    let response = send(&router, "GET", "/app", None, None).await;

    assert!(response.status().is_redirection());
    assert_eq!(
        response.headers().get(header::LOCATION),
        Some(&header::HeaderValue::from_static("/login"))
    );
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn duplicate_registration_and_invalid_logins_fail_closed() {
    let database = db::connect("sqlite::memory:")
        .await
        .expect("in-memory database should open");
    let router = web::router(database, None);
    let (registered, _) = register(&router, "alice@example.test").await;
    assert_eq!(registered.status(), StatusCode::SEE_OTHER);

    let duplicate = serde_urlencoded::to_string(Credentials {
        email: "ALICE@example.test",
        password: "十分に長いパスワードです",
    })
    .expect("credentials should serialize");
    let duplicate = send(&router, "POST", "/register", Some(duplicate), None).await;
    assert_eq!(duplicate.status(), StatusCode::BAD_REQUEST);

    for credentials in [
        Credentials {
            email: "unknown@example.test",
            password: "十分に長いパスワードです",
        },
        Credentials {
            email: "alice@example.test",
            password: "wrong-password-123",
        },
    ] {
        let form = serde_urlencoded::to_string(credentials).expect("credentials should serialize");
        let response = send(&router, "POST", "/login", Some(form), None).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn registration_training_isolation_and_logout_work_as_one_flow() {
    let database = db::connect("sqlite::memory:")
        .await
        .expect("in-memory database should open");
    let router = web::router(database, None);

    let (alice_registration, alice_cookie) = register(&router, "alice@example.test").await;
    assert_eq!(alice_registration.status(), StatusCode::SEE_OTHER);
    let set_cookie = alice_registration
        .headers()
        .get(header::SET_COOKIE)
        .expect("registration should set the session cookie")
        .to_str()
        .expect("cookie should be ASCII");
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Secure"));

    let training = serde_urlencoded::to_string(Training {
        trained_on: "2026-08-12",
        uniform: "gi",
        rounds: 5,
        reflection: "姿勢を崩し切れなかった",
        name: "クローズドガードからの腕十字",
        position: "クローズドガード",
        cue: "肩線を越える",
        answer: "膝で頭の向きを制限する",
        next_try: "三角絞めへつなぐ",
    })
    .expect("training should serialize");
    let saved = send(
        &router,
        "POST",
        "/training-sessions",
        Some(training),
        Some(&alice_cookie),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::SEE_OTHER);

    let alice_app = send(&router, "GET", "/app", None, Some(&alice_cookie)).await;
    assert_eq!(alice_app.status(), StatusCode::OK);
    let alice_html = body_text(alice_app).await;
    assert!(alice_html.contains("クローズドガードからの腕十字"));
    assert!(alice_html.contains("肩線を越える"));
    assert!(alice_html.contains("膝で頭の向きを制限する"));

    let (_, bob_cookie) = register(&router, "bob@example.test").await;
    let bob_app = send(&router, "GET", "/app", None, Some(&bob_cookie)).await;
    assert_eq!(bob_app.status(), StatusCode::OK);
    assert!(!body_text(bob_app).await.contains("膝で頭の向きを制限する"));

    let logged_out = send(&router, "POST", "/logout", None, Some(&alice_cookie)).await;
    assert_eq!(logged_out.status(), StatusCode::SEE_OTHER);
    let stale_session = send(&router, "GET", "/app", None, Some(&alice_cookie)).await;
    assert!(stale_session.status().is_redirection());

    let login = serde_urlencoded::to_string(Credentials {
        email: "alice@example.test",
        password: "十分に長いパスワードです",
    })
    .expect("credentials should serialize");
    let logged_in = send(&router, "POST", "/login", Some(login), None).await;
    assert_eq!(logged_in.status(), StatusCode::SEE_OTHER);
    let fresh_cookie = session_cookie(&logged_in);
    let restored_app = send(&router, "GET", "/app", None, Some(&fresh_cookie)).await;
    assert!(
        body_text(restored_app)
            .await
            .contains("クローズドガードからの腕十字")
    );
}

#[tokio::test]
async fn state_changing_cross_site_requests_are_rejected() {
    let database = db::connect("sqlite::memory:")
        .await
        .expect("in-memory database should open");
    let router = web::router(database, None);
    let (_, cookie) = register(&router, "alice@example.test").await;
    let form = serde_urlencoded::to_string(Training {
        trained_on: "2026-08-12",
        uniform: "no_gi",
        rounds: 3,
        reflection: "",
        name: "ギロチン",
        position: "フロントヘッドロック",
        cue: "肩をかぶせる",
        answer: "肘を脇へ寄せる",
        next_try: "",
    })
    .expect("training should serialize");
    let request = Request::builder()
        .method("POST")
        .uri("/training-sessions")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::COOKIE, cookie)
        .header("sec-fetch-site", "cross-site")
        .body(Body::from(form))
        .expect("request should be valid");

    let response = router.handle(request).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
