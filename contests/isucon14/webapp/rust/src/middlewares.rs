use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::CookieJar;

use crate::models::{Chair, Owner, User};
use crate::{AppState, Error};

pub async fn app_auth_middleware(
    State(AppState {
        pool, auth_cache, ..
    }): State<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, Error> {
    let Some(c) = jar.get("app_session") else {
        return Err(Error::Unauthorized("app_session cookie is required"));
    };
    let access_token = c.value();
    let user = if let Some(user) = auth_cache.user(access_token) {
        user
    } else {
        let Some(user): Option<User> = sqlx::query_as("SELECT * FROM users WHERE access_token = ?")
            .bind(access_token)
            .fetch_optional(&pool)
            .await?
        else {
            return Err(Error::Unauthorized("invalid access token"));
        };
        auth_cache.insert_user(user.clone());
        user
    };

    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

pub async fn owner_auth_middleware(
    State(AppState {
        pool, auth_cache, ..
    }): State<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, Error> {
    let Some(c) = jar.get("owner_session") else {
        return Err(Error::Unauthorized("owner_session cookie is required"));
    };
    let access_token = c.value();
    let owner = if let Some(owner) = auth_cache.owner(access_token) {
        owner
    } else {
        let Some(owner): Option<Owner> =
            sqlx::query_as("SELECT * FROM owners WHERE access_token = ?")
                .bind(access_token)
                .fetch_optional(&pool)
                .await?
        else {
            return Err(Error::Unauthorized("invalid access token"));
        };
        auth_cache.insert_owner(owner.clone());
        owner
    };

    req.extensions_mut().insert(owner);

    Ok(next.run(req).await)
}

pub async fn chair_auth_middleware(
    State(AppState {
        pool, auth_cache, ..
    }): State<AppState>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, Error> {
    let Some(c) = jar.get("chair_session") else {
        return Err(Error::Unauthorized("chair_session cookie is required"));
    };
    let access_token = c.value();
    let chair = if let Some(chair) = auth_cache.chair(access_token) {
        chair
    } else {
        let Some(chair): Option<Chair> =
            sqlx::query_as("SELECT * FROM chairs WHERE access_token = ?")
                .bind(access_token)
                .fetch_optional(&pool)
                .await?
        else {
            return Err(Error::Unauthorized("invalid access token"));
        };
        auth_cache.insert_chair(chair.clone());
        chair
    };

    req.extensions_mut().insert(chair);

    Ok(next.run(req).await)
}
