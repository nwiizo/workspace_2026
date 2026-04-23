use axum::{
    Router,
    extract::{Form, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use html_escape::{encode_double_quoted_attribute, encode_text};
use rusqlite::{Connection, params};
use serde::Deserialize;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    download_dir: PathBuf,
}

#[derive(Debug)]
struct Book {
    id: i64,
    title: String,
    author: String,
    status: String,
}

#[derive(Debug)]
struct Patron {
    username: String,
    role: String,
}

#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
}

#[derive(Deserialize)]
struct BookParams {
    id: Option<String>,
}

#[derive(Deserialize)]
struct DownloadParams {
    file: Option<String>,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    csrf_token: String,
}

#[derive(Deserialize)]
struct AccountForm {
    display_name: String,
    csrf_token: String,
}

#[derive(Deserialize)]
struct LoanForm {
    book_id: String,
    borrower: String,
    csrf_token: String,
}

const CSRF_TOKEN: &str = "local-demo-csrf-token";

const STYLE: &str = r#"
:root {
  color-scheme: light;
  font-family: Arial, Helvetica, sans-serif;
  background: #f4f7fb;
  color: #172033;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
}

header {
  align-items: center;
  background: #ffffff;
  border-bottom: 1px solid #d8dee9;
  display: flex;
  gap: 24px;
  justify-content: space-between;
  padding: 16px 24px;
}

a {
  color: #075985;
}

.brand {
  color: #172033;
  font-weight: 700;
  text-decoration: none;
}

nav {
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
}

main {
  margin: 0 auto;
  max-width: 980px;
  padding: 32px 24px;
}

h1 {
  font-size: 32px;
  margin: 0 0 20px;
}

h2 {
  font-size: 20px;
  margin: 0 0 8px;
}

.intro {
  align-items: end;
  display: flex;
  gap: 24px;
  justify-content: space-between;
  margin-bottom: 24px;
}

.search-form,
.stack {
  display: flex;
  gap: 12px;
}

.stack {
  align-items: start;
  flex-direction: column;
  max-width: 380px;
}

label {
  display: grid;
  gap: 6px;
  width: 100%;
}

input,
select {
  border: 1px solid #a8b3c5;
  border-radius: 6px;
  font: inherit;
  min-height: 40px;
  padding: 8px 10px;
}

button {
  background: #14532d;
  border: 0;
  border-radius: 6px;
  color: #ffffff;
  cursor: pointer;
  font: inherit;
  min-height: 40px;
  padding: 8px 14px;
}

.grid {
  display: grid;
  gap: 16px;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
}

article {
  background: #ffffff;
  border: 1px solid #d8dee9;
  border-radius: 8px;
  padding: 18px;
}

pre {
  background: #111827;
  border-radius: 8px;
  color: #e5e7eb;
  overflow: auto;
  padding: 16px;
}

.error {
  color: #b91c1c;
  font-weight: 700;
}

@media (max-width: 720px) {
  header,
  .intro,
  .search-form {
    align-items: stretch;
    flex-direction: column;
  }
}
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState {
        db: Arc::new(Mutex::new(init_db()?)),
        download_dir: PathBuf::from("files"),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/account", get(account).post(update_account))
        .route("/book", get(book))
        .route("/download", get(download))
        .route("/health", get(health))
        .route("/loans", post(create_loan))
        .route("/login", get(login_page).post(login))
        .route("/search", get(search))
        .route("/static/style.css", get(style))
        .with_state(state)
        .layer(middleware::map_response(add_security_headers));

    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_db() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        r#"
        CREATE TABLE patrons (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            role TEXT NOT NULL
        );

        CREATE TABLE books (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            author TEXT NOT NULL,
            status TEXT NOT NULL
        );

        INSERT INTO patrons (username, password, role) VALUES
            ('librarian', 'library123', 'staff'),
            ('alice', 'password', 'member');

        INSERT INTO books (title, author, status) VALUES
            ('Threat Modeling for Small Teams', 'A. Shiraishi', 'available'),
            ('Secure Rust Services', 'M. Tanaka', 'available'),
            ('OAuth Field Notes', 'R. Sato', 'checked out'),
            ('Practical Access Control', 'K. Mori', 'available');
        "#,
    )?;
    Ok(conn)
}

async fn health() -> &'static str {
    "ok"
}

async fn style() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], STYLE)
}

async fn index(State(state): State<AppState>) -> Response {
    let rows = match list_books(&state) {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("catalog lookup failed: {error}");
            return server_error("Catalog Error", "catalog-query").into_response();
        }
    };

    let books = rows
        .into_iter()
        .map(|book| {
            format!(
                r#"<article>
                    <h2>{}</h2>
                    <p>{}</p>
                    <p>Status: {}</p>
                    <a href="/book?id={}">View record</a>
                  </article>"#,
                escape_text(&book.title),
                escape_text(&book.author),
                escape_text(&book.status),
                book.id
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let content = format!(
        r#"
        <!-- TODO: remove demo credentials before launch: librarian/library123 -->
        <section class="intro">
          <h1>Library Loans</h1>
          <form action="/search" method="get" class="search-form">
            <input name="q" value="rust" aria-label="Search query">
            <button type="submit">Search</button>
          </form>
        </section>
        <section class="grid">{books}</section>
        <section>
          <h2>Request a Loan</h2>
          <form action="/loans" method="post" class="stack">
            <label>
              Book ID
              <input name="book_id" value="1">
            </label>
            <label>
              Borrower
              <input name="borrower" value="alice">
            </label>
            <input type="hidden" name="csrf_token" value="{CSRF_TOKEN}">
            <button type="submit">Request loan</button>
          </form>
        </section>
        "#
    );

    layout("Library Loans", content).into_response()
}

async fn search(Query(params): Query<SearchParams>) -> Response {
    let query = params.q.unwrap_or_default();
    let query_text = escape_text(&query);
    let query_attr = escape_attr(&query);
    let result = if query.is_empty() {
        "Enter a search term.".to_string()
    } else {
        format!("Results for: {query_text}")
    };

    let content = format!(
        r#"
        <section>
          <h1>Search Catalog</h1>
          <form action="/search" method="get" class="search-form">
            <input name="q" value="{query_attr}" aria-label="Search query">
            <button type="submit">Search</button>
          </form>
          <p>{result}</p>
        </section>
        "#
    );

    layout("Search Catalog", content).into_response()
}

async fn book(State(state): State<AppState>, Query(params): Query<BookParams>) -> Response {
    let id = match params.id.as_deref().unwrap_or("1").parse::<i64>() {
        Ok(id) => id,
        Err(_) => return bad_request("Invalid book id.").into_response(),
    };

    let rows = match find_books_by_id(&state, id) {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("book lookup failed: {error}");
            return server_error("Database Error", "book-query").into_response();
        }
    };

    let records = if rows.is_empty() {
        "<p>No book found.</p>".to_string()
    } else {
        rows.into_iter()
            .map(|book| {
                format!(
                    r#"<article>
                        <h2>{}</h2>
                        <p>Author: {}</p>
                        <p>Status: {}</p>
                      </article>"#,
                    escape_text(&book.title),
                    escape_text(&book.author),
                    escape_text(&book.status)
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };

    let content = format!(
        r#"
        <section>
          <h1>Book Record</h1>
          <div class="grid">{records}</div>
        </section>
        "#
    );

    layout("Book Record", content).into_response()
}

async fn login_page() -> Response {
    login_form(None).into_response()
}

async fn login(State(state): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    if !valid_csrf(&form.csrf_token) {
        return bad_request("Invalid CSRF token.").into_response();
    }

    let patron = match find_patron(&state, &form.username, &form.password) {
        Ok(patron) => patron,
        Err(error) => {
            eprintln!("login lookup failed: {error}");
            return server_error("Database Error", "login-query").into_response();
        }
    };

    if let Some(patron) = patron {
        let mut response = Redirect::to("/account").into_response();
        let cookie = format!(
            "library_session={}-{}-session; Path=/; HttpOnly; SameSite=Lax",
            patron.username, patron.role
        );
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            response.headers_mut().insert(header::SET_COOKIE, value);
        }
        return response;
    }

    login_form(Some("Invalid username or password.")).into_response()
}

async fn account(headers: HeaderMap) -> Response {
    let session =
        current_session(&headers).unwrap_or_else(|| "No session cookie is set.".to_string());
    let session = escape_text(&session);
    let content = format!(
        r#"
        <section>
          <h1>Patron Account</h1>
          <p>Session: {session}</p>
          <form action="/account" method="post" class="stack">
            <label>
              Display name
              <input name="display_name" value="Alice Reader">
            </label>
            <input type="hidden" name="csrf_token" value="{CSRF_TOKEN}">
            <button type="submit">Save profile</button>
          </form>
        </section>
        "#
    );

    layout("Patron Account", content).into_response()
}

async fn update_account(headers: HeaderMap, Form(form): Form<AccountForm>) -> Response {
    if !valid_csrf(&form.csrf_token) {
        return bad_request("Invalid CSRF token.").into_response();
    }

    let session =
        current_session(&headers).unwrap_or_else(|| "No session cookie is set.".to_string());
    let session = escape_text(&session);
    let display_name_text = escape_text(&form.display_name);
    let display_name_attr = escape_attr(&form.display_name);
    let content = format!(
        r#"
        <section>
          <h1>Patron Account</h1>
          <p>Session: {session}</p>
          <p>Saved display name: {display_name_text}</p>
          <form action="/account" method="post" class="stack">
            <label>
              Display name
              <input name="display_name" value="{display_name_attr}">
            </label>
            <input type="hidden" name="csrf_token" value="{CSRF_TOKEN}">
            <button type="submit">Save profile</button>
          </form>
        </section>
        "#
    );

    layout("Patron Account", content).into_response()
}

async fn create_loan(Form(form): Form<LoanForm>) -> Response {
    if !valid_csrf(&form.csrf_token) {
        return bad_request("Invalid CSRF token.").into_response();
    }

    let book_id = escape_text(&form.book_id);
    let borrower = escape_text(&form.borrower);
    let content = format!(
        r#"
        <section>
          <h1>Loan Requested</h1>
          <p>Book ID: {book_id}</p>
          <p>Borrower: {borrower}</p>
          <a href="/">Back to catalog</a>
        </section>
        "#
    );

    layout("Loan Requested", content).into_response()
}

async fn download(State(state): State<AppState>, Query(params): Query<DownloadParams>) -> Response {
    let requested = params.file.as_deref().unwrap_or("borrowing-policy.txt");
    let file_name = match requested {
        "borrowing-policy.txt" => "borrowing-policy.txt",
        _ => return (StatusCode::NOT_FOUND, "file not found").into_response(),
    };
    let target = state.download_dir.join(file_name);

    match tokio::fs::read(&target).await {
        Ok(bytes) => {
            let download_name = Path::new(file_name)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("download.txt");
            let mut response = bytes.into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            if let Ok(value) =
                HeaderValue::from_str(&format!("attachment; filename=\"{download_name}\""))
            {
                response
                    .headers_mut()
                    .insert(header::CONTENT_DISPOSITION, value);
            }
            response
        }
        Err(error) => {
            eprintln!("download failed: {error}");
            (StatusCode::NOT_FOUND, "file not found").into_response()
        }
    }
}

fn list_books(state: &AppState) -> rusqlite::Result<Vec<Book>> {
    let conn = state.db.lock().expect("database mutex poisoned");
    let mut statement = conn.prepare("SELECT id, title, author, status FROM books ORDER BY id")?;
    let rows = statement.query_map([], map_book)?;
    rows.collect()
}

fn find_books_by_id(state: &AppState, id: i64) -> rusqlite::Result<Vec<Book>> {
    let conn = state.db.lock().expect("database mutex poisoned");
    let mut statement =
        conn.prepare("SELECT id, title, author, status FROM books WHERE id = ?1")?;
    let rows = statement.query_map(params![id], map_book)?;
    rows.collect()
}

fn find_patron(
    state: &AppState,
    username: &str,
    password: &str,
) -> rusqlite::Result<Option<Patron>> {
    let conn = state.db.lock().expect("database mutex poisoned");
    let mut statement =
        conn.prepare("SELECT username, role FROM patrons WHERE username = ?1 AND password = ?2")?;
    let mut rows = statement.query_map(params![username, password], |row| {
        Ok(Patron {
            username: row.get(0)?,
            role: row.get(1)?,
        })
    })?;

    rows.next().transpose()
}

fn map_book(row: &rusqlite::Row<'_>) -> rusqlite::Result<Book> {
    Ok(Book {
        id: row.get(0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        status: row.get(3)?,
    })
}

fn login_form(message: Option<&str>) -> Html<String> {
    let message_html = message
        .map(|value| format!(r#"<p class="error">{}</p>"#, escape_text(value)))
        .unwrap_or_default();
    let content = format!(
        r#"
        <section>
          <h1>Staff Login</h1>
          {message_html}
          <form action="/login" method="post" class="stack">
            <label>
              Username
              <input name="username" autocomplete="username">
            </label>
            <label>
              Password
              <input name="password" type="password" autocomplete="current-password">
            </label>
            <input type="hidden" name="csrf_token" value="{CSRF_TOKEN}">
            <button type="submit">Login</button>
          </form>
        </section>
        "#
    );

    layout("Staff Login", content)
}

fn server_error(title: &str, request_id: &str) -> (StatusCode, Html<String>) {
    let title = escape_text(title);
    let request_id = escape_text(request_id);
    let content = format!(
        r#"
        <section>
          <h1>{title}</h1>
          <p class="error">Request failed. Reference: {request_id}</p>
        </section>
        "#
    );

    (StatusCode::INTERNAL_SERVER_ERROR, layout(&title, content))
}

fn bad_request(message: &str) -> (StatusCode, Html<String>) {
    let message = escape_text(message);
    let content = format!(
        r#"
        <section>
          <h1>Bad Request</h1>
          <p class="error">{message}</p>
        </section>
        "#
    );

    (StatusCode::BAD_REQUEST, layout("Bad Request", content))
}

fn valid_csrf(token: &str) -> bool {
    token == CSRF_TOKEN
}

fn escape_text(value: &str) -> String {
    encode_text(value).into_owned()
}

fn escape_attr(value: &str) -> String {
    encode_double_quoted_attribute(value).into_owned()
}

fn current_session(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let trimmed = part.trim();
        trimmed
            .strip_prefix("library_session=")
            .map(ToOwned::to_owned)
    })
}

fn layout(title: &str, content: String) -> Html<String> {
    let title = escape_text(title);
    Html(format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <link rel="stylesheet" href="/static/style.css">
  </head>
  <body>
    <header>
      <a class="brand" href="/">Rust Library Loans</a>
      <nav>
        <a href="/search?q=rust">Search</a>
        <a href="/book?id=1">Book</a>
        <a href="/download?file=borrowing-policy.txt">Policy</a>
        <a href="/login">Login</a>
        <a href="/account">Account</a>
      </nav>
    </header>
    <main>{content}</main>
  </body>
	</html>"#
    ))
}

async fn add_security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'",
        ),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), geolocation=(), microphone=()"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-embedder-policy"),
        HeaderValue::from_static("require-corp"),
    );
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    response
}
