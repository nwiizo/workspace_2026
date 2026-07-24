use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use ulid::Ulid;

use crate::models::{Chair, Coupon, Owner, PaymentToken, Ride, RideStatus, User};
use crate::{AppState, Coordinate, Error};

pub fn app_routes(app_state: AppState) -> axum::Router<AppState> {
    let routes = axum::Router::new().route("/api/app/users", axum::routing::post(app_post_users));

    let authed_routes = axum::Router::new()
        .route(
            "/api/app/payment-methods",
            axum::routing::post(app_post_payment_methods),
        )
        .route(
            "/api/app/rides",
            axum::routing::get(app_get_rides).post(app_post_rides),
        )
        .route(
            "/api/app/rides/estimated-fare",
            axum::routing::post(app_post_rides_estimated_fare),
        )
        .route(
            "/api/app/rides/:ride_id/evaluation",
            axum::routing::post(app_post_ride_evaluation),
        )
        .route(
            "/api/app/notification",
            axum::routing::get(app_get_notification),
        )
        .route(
            "/api/app/nearby-chairs",
            axum::routing::get(app_get_nearby_chairs),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            crate::middlewares::app_auth_middleware,
        ));

    routes.merge(authed_routes)
}

#[derive(Debug, serde::Deserialize)]
struct AppPostUsersRequest {
    username: String,
    firstname: String,
    lastname: String,
    date_of_birth: String,
    invitation_code: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct AppPostUsersResponse {
    id: String,
    invitation_code: String,
}

async fn app_post_users(
    State(AppState { pool, .. }): State<AppState>,
    jar: CookieJar,
    axum::Json(req): axum::Json<AppPostUsersRequest>,
) -> Result<(CookieJar, (StatusCode, axum::Json<AppPostUsersResponse>)), Error> {
    let user_id = Ulid::new().to_string();
    let access_token = crate::secure_random_str(32);
    let invitation_code = crate::secure_random_str(15);

    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO users (id, username, firstname, lastname, date_of_birth, access_token, invitation_code) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&user_id)
        .bind(req.username)
        .bind(req.firstname)
        .bind(req.lastname)
        .bind(req.date_of_birth)
        .bind(&access_token)
        .bind(&invitation_code)
        .execute(&mut *tx)
        .await?;

    // 初回登録キャンペーンのクーポンを付与
    sqlx::query("INSERT INTO coupons (user_id, code, discount) VALUES (?, ?, ?)")
        .bind(&user_id)
        .bind("CP_NEW2024")
        .bind(3000)
        .execute(&mut *tx)
        .await?;

    // 招待コードを使った登録
    if let Some(req_invitation_code) = req.invitation_code {
        if !req_invitation_code.is_empty() {
            // 招待する側の招待数をチェック
            let coupons: Vec<Coupon> =
                sqlx::query_as("SELECT * FROM coupons WHERE code = ? FOR UPDATE")
                    .bind(format!("INV_{req_invitation_code}"))
                    .fetch_all(&mut *tx)
                    .await?;
            if coupons.len() >= 3 {
                return Err(Error::BadRequest("この招待コードは使用できません。"));
            }

            // ユーザーチェック
            let Some(inviter): Option<User> =
                sqlx::query_as("SELECT * FROM users WHERE invitation_code = ?")
                    .bind(&req_invitation_code)
                    .fetch_optional(&mut *tx)
                    .await?
            else {
                return Err(Error::BadRequest("この招待コードは使用できません。"));
            };

            // 招待クーポン付与
            sqlx::query("INSERT INTO coupons (user_id, code, discount) VALUES (?, ?, ?)")
                .bind(&user_id)
                .bind(format!("INV_{req_invitation_code}"))
                .bind(1500)
                .execute(&mut *tx)
                .await?;
            // 招待した人にもRewardを付与
            sqlx::query("INSERT INTO coupons (user_id, code, discount) VALUES (?, CONCAT(?, '_', FLOOR(UNIX_TIMESTAMP(NOW(3))*1000)), ?)")
                .bind(inviter.id)
                .bind(format!("RWD_{req_invitation_code}"))
                .bind(1000)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;

    let jar = jar
        .add(axum_extra::extract::cookie::Cookie::build(("app_session", access_token)).path("/"));

    Ok((
        jar,
        (
            StatusCode::CREATED,
            axum::Json(AppPostUsersResponse {
                id: user_id,
                invitation_code,
            }),
        ),
    ))
}

#[derive(Debug, serde::Deserialize)]
struct AppPostPaymentMethodsRequest {
    token: String,
}

async fn app_post_payment_methods(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
    axum::Json(req): axum::Json<AppPostPaymentMethodsRequest>,
) -> Result<StatusCode, Error> {
    sqlx::query("INSERT INTO payment_tokens (user_id, token) VALUES (?, ?)")
        .bind(user.id)
        .bind(req.token)
        .execute(&pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Serialize)]
struct GetAppRidesResponse {
    rides: Vec<GetAppRidesResponseItem>,
}

#[derive(Debug, serde::Serialize)]
struct GetAppRidesResponseItem {
    id: String,
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
    chair: GetAppRidesResponseItemChair,
    fare: i32,
    evaluation: i32,
    requested_at: i64,
    completed_at: i64,
}

#[derive(Debug, serde::Serialize)]
struct GetAppRidesResponseItemChair {
    id: String,
    owner: String,
    name: String,
    model: String,
}

async fn app_get_rides(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
) -> Result<axum::Json<GetAppRidesResponse>, Error> {
    let mut tx = pool.begin().await?;

    let rides: Vec<Ride> =
        sqlx::query_as("SELECT * FROM rides WHERE user_id = ? ORDER BY created_at DESC")
            .bind(&user.id)
            .fetch_all(&mut *tx)
            .await?;

    let mut items = Vec::with_capacity(rides.len());
    for ride in rides {
        let status = crate::get_latest_ride_status(&mut *tx, &ride.id).await?;
        if status != "COMPLETED" {
            continue;
        }

        let fare = calculate_discounted_fare(
            &mut tx,
            &user.id,
            Some(&ride),
            ride.pickup_latitude,
            ride.pickup_longitude,
            ride.destination_latitude,
            ride.destination_longitude,
        )
        .await?;

        let chair: Chair = sqlx::query_as("SELECT * FROM chairs WHERE id = ?")
            .bind(&ride.chair_id)
            .fetch_one(&mut *tx)
            .await?;

        let owner: Owner = sqlx::query_as("SELECT * FROM owners WHERE id = ?")
            .bind(chair.owner_id)
            .fetch_one(&mut *tx)
            .await?;

        items.push(GetAppRidesResponseItem {
            id: ride.id,
            pickup_coordinate: Coordinate {
                latitude: ride.pickup_latitude,
                longitude: ride.pickup_longitude,
            },
            destination_coordinate: Coordinate {
                latitude: ride.destination_latitude,
                longitude: ride.destination_longitude,
            },
            chair: GetAppRidesResponseItemChair {
                id: chair.id,
                owner: owner.name,
                name: chair.name,
                model: chair.model,
            },
            fare,
            evaluation: ride.evaluation.unwrap(),
            requested_at: ride.created_at.timestamp_millis(),
            completed_at: ride.updated_at.timestamp_millis(),
        });
    }

    tx.commit().await?;

    Ok(axum::Json(GetAppRidesResponse { rides: items }))
}

#[derive(Debug, serde::Deserialize)]
struct AppPostRidesRequest {
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
}

#[derive(Debug, serde::Serialize)]
struct AppPostRidesResponse {
    ride_id: String,
    fare: i32,
}

async fn app_post_rides(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
    axum::Json(req): axum::Json<AppPostRidesRequest>,
) -> Result<(StatusCode, axum::Json<AppPostRidesResponse>), Error> {
    let ride_id = Ulid::new().to_string();

    let mut tx = pool.begin().await?;

    let rides: Vec<Ride> = sqlx::query_as("SELECT * FROM rides WHERE user_id = ?")
        .bind(&user.id)
        .fetch_all(&mut *tx)
        .await?;

    let mut continuing_ride_count = 0;
    for ride in rides {
        let status = crate::get_latest_ride_status(&mut *tx, &ride.id).await?;
        if status != "COMPLETED" {
            continuing_ride_count += 1;
        }
    }

    if continuing_ride_count > 0 {
        return Err(Error::Conflict("ride already exists"));
    }

    sqlx::query("INSERT INTO rides (id, user_id, pickup_latitude, pickup_longitude, destination_latitude, destination_longitude) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&ride_id)
        .bind(&user.id)
        .bind(req.pickup_coordinate.latitude)
        .bind(req.pickup_coordinate.longitude)
        .bind(req.destination_coordinate.latitude)
        .bind(req.destination_coordinate.longitude)
        .execute(&mut *tx)
        .await?;

    sqlx::query("INSERT INTO ride_statuses (id, ride_id, status) VALUES (?, ?, ?)")
        .bind(Ulid::new().to_string())
        .bind(&ride_id)
        .bind("MATCHING")
        .execute(&mut *tx)
        .await?;

    let ride_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rides WHERE user_id = ?")
        .bind(&user.id)
        .fetch_one(&mut *tx)
        .await?;

    if ride_count == 1 {
        // 初回利用で、初回利用クーポンがあれば必ず使う
        let coupon: Option<Coupon> = sqlx::query_as("SELECT * FROM coupons WHERE user_id = ? AND code = 'CP_NEW2024' AND used_by IS NULL FOR UPDATE")
            .bind(&user.id)
            .fetch_optional(&mut *tx)
            .await?;
        if coupon.is_some() {
            sqlx::query("UPDATE coupons SET used_by = ? WHERE user_id = ? AND code = 'CP_NEW2024'")
                .bind(&ride_id)
                .bind(&user.id)
                .execute(&mut *tx)
                .await?;
        } else {
            // 無ければ他のクーポンを付与された順番に使う
            let coupon: Option<Coupon> = sqlx::query_as("SELECT * FROM coupons WHERE user_id = ? AND used_by IS NULL ORDER BY created_at LIMIT 1 FOR UPDATE")
                .bind(&user.id)
                .fetch_optional(&mut *tx)
                .await?;
            if let Some(coupon) = coupon {
                sqlx::query("UPDATE coupons SET used_by = ? WHERE user_id = ? AND code = ?")
                    .bind(&ride_id)
                    .bind(&user.id)
                    .bind(coupon.code)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    } else {
        // 他のクーポンを付与された順番に使う
        let coupon: Option<Coupon> = sqlx::query_as("SELECT * FROM coupons WHERE user_id = ? AND used_by IS NULL ORDER BY created_at LIMIT 1 FOR UPDATE")
                .bind(&user.id)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some(coupon) = coupon {
            sqlx::query("UPDATE coupons SET used_by = ? WHERE user_id = ? AND code = ?")
                .bind(&ride_id)
                .bind(&user.id)
                .bind(coupon.code)
                .execute(&mut *tx)
                .await?;
        }
    }

    let ride: Ride = sqlx::query_as("SELECT * FROM rides WHERE id = ?")
        .bind(&ride_id)
        .fetch_one(&mut *tx)
        .await?;

    let fare = calculate_discounted_fare(
        &mut tx,
        &user.id,
        Some(&ride),
        req.pickup_coordinate.latitude,
        req.pickup_coordinate.longitude,
        req.destination_coordinate.latitude,
        req.destination_coordinate.longitude,
    )
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::ACCEPTED,
        axum::Json(AppPostRidesResponse { ride_id, fare }),
    ))
}

#[derive(Debug, serde::Deserialize)]
struct AppPostRidesEstimatedFareRequest {
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
}

#[derive(Debug, serde::Serialize)]
struct AppPostRidesEstimatedFareResponse {
    fare: i32,
    discount: i32,
}

async fn app_post_rides_estimated_fare(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
    axum::Json(req): axum::Json<AppPostRidesEstimatedFareRequest>,
) -> Result<axum::Json<AppPostRidesEstimatedFareResponse>, Error> {
    let mut tx = pool.begin().await?;

    let discounted = calculate_discounted_fare(
        &mut tx,
        &user.id,
        None,
        req.pickup_coordinate.latitude,
        req.pickup_coordinate.longitude,
        req.destination_coordinate.latitude,
        req.destination_coordinate.longitude,
    )
    .await?;

    tx.commit().await?;

    Ok(axum::Json(AppPostRidesEstimatedFareResponse {
        fare: discounted,
        discount: crate::calculate_fare(
            req.pickup_coordinate.latitude,
            req.pickup_coordinate.longitude,
            req.destination_coordinate.latitude,
            req.destination_coordinate.longitude,
        ) - discounted,
    }))
}

#[derive(Debug, serde::Deserialize)]
struct AppPostRideEvaluationRequest {
    evaluation: i32,
}

#[derive(Debug, serde::Serialize)]
struct AppPostRideEvaluationResponse {
    fare: i32,
    completed_at: i64,
}

async fn app_post_ride_evaluation(
    State(AppState { pool, .. }): State<AppState>,
    Path((ride_id,)): Path<(String,)>,
    axum::Json(req): axum::Json<AppPostRideEvaluationRequest>,
) -> Result<axum::Json<AppPostRideEvaluationResponse>, Error> {
    if req.evaluation < 1 || req.evaluation > 5 {
        return Err(Error::BadRequest("evaluation must be between 1 and 5"));
    }

    let mut tx = pool.begin().await?;

    let Some(ride): Option<Ride> = sqlx::query_as("SELECT * FROM rides WHERE id = ?")
        .bind(&ride_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Err(Error::NotFound("ride not found"));
    };
    let status = crate::get_latest_ride_status(&mut *tx, &ride.id).await?;

    if status != "ARRIVED" {
        return Err(Error::BadRequest("not arrived yet"));
    }

    let result = sqlx::query("UPDATE rides SET evaluation = ? WHERE id = ?")
        .bind(req.evaluation)
        .bind(&ride_id)
        .execute(&mut *tx)
        .await?;
    let count = result.rows_affected();
    if count == 0 {
        return Err(Error::NotFound("ride not found"));
    }

    sqlx::query("INSERT INTO ride_statuses (id, ride_id, status) VALUES (?, ?, ?)")
        .bind(Ulid::new().to_string())
        .bind(&ride_id)
        .bind("COMPLETED")
        .execute(&mut *tx)
        .await?;

    let Some(ride): Option<Ride> = sqlx::query_as("SELECT * FROM rides WHERE id = ?")
        .bind(&ride_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Err(Error::NotFound("ride not found"));
    };

    let Some(payment_token): Option<PaymentToken> =
        sqlx::query_as("SELECT * FROM payment_tokens WHERE user_id = ?")
            .bind(&ride.user_id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return Err(Error::BadRequest("payment token not registered"));
    };

    let fare = calculate_discounted_fare(
        &mut tx,
        &ride.user_id,
        Some(&ride),
        ride.pickup_latitude,
        ride.pickup_longitude,
        ride.destination_latitude,
        ride.destination_longitude,
    )
    .await?;

    let payment_gateway_url: String =
        sqlx::query_scalar("SELECT value FROM settings WHERE name = 'payment_gateway_url'")
            .fetch_one(&mut *tx)
            .await?;

    async fn retrieve_rides_order_by_created_at_asc(
        tx: &mut sqlx::MySqlConnection,
        user_id: &str,
    ) -> Result<Vec<Ride>, Error> {
        sqlx::query_as("SELECT * FROM rides WHERE user_id = ? ORDER BY created_at ASC")
            .bind(user_id)
            .fetch_all(tx)
            .await
            .map_err(Error::Sqlx)
    }

    crate::payment_gateway::request_payment_gateway_post_payment(
        &payment_gateway_url,
        &payment_token.token,
        &crate::payment_gateway::PaymentGatewayPostPaymentRequest { amount: fare },
        &mut tx,
        &ride.user_id,
        retrieve_rides_order_by_created_at_asc,
    )
    .await?;

    tx.commit().await?;

    Ok(axum::Json(AppPostRideEvaluationResponse {
        fare,
        completed_at: ride.updated_at.timestamp_millis(),
    }))
}

#[derive(Debug, serde::Serialize)]
struct AppGetNotificationResponse {
    data: Option<AppGetNotificationResponseData>,
    retry_after_ms: Option<i32>,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNotificationResponseData {
    ride_id: String,
    pickup_coordinate: Coordinate,
    destination_coordinate: Coordinate,
    fare: i32,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    chair: Option<AppGetNotificationResponseChair>,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNotificationResponseChair {
    id: String,
    name: String,
    model: String,
    stats: AppGetNotificationResponseChairStats,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNotificationResponseChairStats {
    total_rides_count: i32,
    total_evaluation_avg: f64,
}

async fn app_get_notification(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(user): axum::Extension<User>,
) -> Result<axum::Json<AppGetNotificationResponse>, Error> {
    let ride_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM rides WHERE user_id = ? ORDER BY created_at DESC LIMIT 1")
            .bind(&user.id)
            .fetch_optional(&pool)
            .await?;
    if ride_exists.is_none() {
        return Ok(axum::Json(AppGetNotificationResponse {
            data: None,
            retry_after_ms: Some(30),
        }));
    }

    // 通知内容を組み立てる複数の SELECT と通知済み更新は、同じ
    // スナップショットで扱う必要がある。ライドがない利用者だけを
    // トランザクション開始前に返し、整合性と負荷削減を両立する。
    let mut tx = pool.begin().await?;
    let ride: Ride =
        sqlx::query_as("SELECT * FROM rides WHERE user_id = ? ORDER BY created_at DESC LIMIT 1")
            .bind(&user.id)
            .fetch_one(&mut *tx)
            .await?;

    let yet_sent_ride_status: Option<RideStatus> = sqlx::query_as("SELECT * FROM ride_statuses WHERE ride_id = ? AND app_sent_at IS NULL ORDER BY created_at ASC LIMIT 1")
        .bind(&ride.id)
        .fetch_optional(&mut *tx)
        .await?;
    let (ride_status_id, status) = if let Some(yet_sent_ride_status) = yet_sent_ride_status {
        (Some(yet_sent_ride_status.id), yet_sent_ride_status.status)
    } else {
        (
            None,
            crate::get_latest_ride_status(&mut *tx, &ride.id).await?,
        )
    };

    let fare = calculate_discounted_fare(
        &mut tx,
        &user.id,
        Some(&ride),
        ride.pickup_latitude,
        ride.pickup_longitude,
        ride.destination_latitude,
        ride.destination_longitude,
    )
    .await?;

    let mut data = AppGetNotificationResponseData {
        ride_id: ride.id,
        pickup_coordinate: Coordinate {
            latitude: ride.pickup_latitude,
            longitude: ride.pickup_longitude,
        },
        destination_coordinate: Coordinate {
            latitude: ride.destination_latitude,
            longitude: ride.destination_longitude,
        },
        fare,
        status,
        chair: None,
        created_at: ride.created_at.timestamp_millis(),
        updated_at: ride.updated_at.timestamp_millis(),
    };

    if let Some(chair_id) = ride.chair_id {
        let chair: Chair = sqlx::query_as("SELECT * FROM chairs WHERE id = ?")
            .bind(chair_id)
            .fetch_one(&mut *tx)
            .await?;

        let stats = get_chair_stats(&mut tx, &chair.id).await?;

        data.chair = Some(AppGetNotificationResponseChair {
            id: chair.id,
            name: chair.name,
            model: chair.model,
            stats,
        });
    }

    if let Some(ride_status_id) = ride_status_id {
        sqlx::query("UPDATE ride_statuses SET app_sent_at = CURRENT_TIMESTAMP(6) WHERE id = ?")
            .bind(ride_status_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(axum::Json(AppGetNotificationResponse {
        data: Some(data),
        retry_after_ms: Some(30),
    }))
}

async fn get_chair_stats(
    tx: &mut sqlx::MySqlConnection,
    chair_id: &str,
) -> Result<AppGetNotificationResponseChairStats, Error> {
    #[derive(sqlx::FromRow)]
    struct ChairStats {
        total_rides_count: i64,
        total_evaluation_avg: f64,
    }

    let stats: ChairStats = sqlx::query_as(
        r#"
SELECT COUNT(*) AS total_rides_count,
       CAST(COALESCE(AVG(completed_rides.evaluation), 0) AS DOUBLE)
           AS total_evaluation_avg
FROM (
    SELECT rides.id, rides.evaluation
    FROM rides
    INNER JOIN ride_statuses ON ride_statuses.ride_id = rides.id
    WHERE rides.chair_id = ?
      AND rides.evaluation IS NOT NULL
    GROUP BY rides.id, rides.evaluation
    HAVING SUM(ride_statuses.status = 'ARRIVED') > 0
       AND SUM(ride_statuses.status = 'CARRYING') > 0
       AND SUM(ride_statuses.status = 'COMPLETED') > 0
) AS completed_rides
        "#,
    )
    .bind(chair_id)
    .fetch_one(&mut *tx)
    .await?;

    Ok(AppGetNotificationResponseChairStats {
        total_rides_count: stats.total_rides_count as i32,
        total_evaluation_avg: stats.total_evaluation_avg,
    })
}

#[derive(Debug, serde::Deserialize)]
struct AppGetNearbyChairsQuery {
    latitude: i32,
    longitude: i32,
    distance: Option<i32>,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNearbyChairsResponse {
    chairs: Vec<AppGetNearbyChairsResponseChair>,
    retrieved_at: i64,
}

#[derive(Debug, serde::Serialize)]
struct AppGetNearbyChairsResponseChair {
    id: String,
    name: String,
    model: String,
    current_coordinate: Coordinate,
}

#[derive(Debug, sqlx::FromRow)]
struct NearbyChair {
    id: String,
    name: String,
    model: String,
    latitude: i32,
    longitude: i32,
}

async fn app_get_nearby_chairs(
    State(AppState { pool, .. }): State<AppState>,
    Query(query): Query<AppGetNearbyChairsQuery>,
) -> Result<axum::Json<AppGetNearbyChairsResponse>, Error> {
    let distance = query.distance.unwrap_or(50);
    let coordinate = Coordinate {
        latitude: query.latitude,
        longitude: query.longitude,
    };

    let chairs: Vec<NearbyChair> = sqlx::query_as(
        r#"
SELECT chairs.id,
       chairs.name,
       chairs.model,
       latest_location.latitude,
       latest_location.longitude
FROM chairs
INNER JOIN LATERAL (
    SELECT latitude, longitude
    FROM chair_locations
    WHERE chair_id = chairs.id
    ORDER BY created_at DESC
    LIMIT 1
) AS latest_location ON TRUE
WHERE chairs.is_active = TRUE
  AND NOT EXISTS (
      SELECT 1
      FROM rides
      WHERE rides.chair_id = chairs.id
        AND COALESCE((
            SELECT ride_statuses.status
            FROM ride_statuses
            WHERE ride_statuses.ride_id = rides.id
            ORDER BY ride_statuses.created_at DESC
            LIMIT 1
        ), '') <> 'COMPLETED'
  )
        "#,
    )
    .fetch_all(&pool)
    .await?;

    let mut nearby_chairs = Vec::with_capacity(chairs.len());
    for chair in chairs {
        if crate::calculate_distance(
            coordinate.latitude,
            coordinate.longitude,
            chair.latitude,
            chair.longitude,
        ) <= distance
        {
            nearby_chairs.push(AppGetNearbyChairsResponseChair {
                id: chair.id,
                name: chair.name,
                model: chair.model,
                current_coordinate: Coordinate {
                    latitude: chair.latitude,
                    longitude: chair.longitude,
                },
            });
        }
    }

    Ok(axum::Json(AppGetNearbyChairsResponse {
        chairs: nearby_chairs,
        retrieved_at: chrono::Utc::now().timestamp_millis(),
    }))
}

async fn calculate_discounted_fare(
    tx: &mut sqlx::MySqlConnection,
    user_id: &str,
    ride: Option<&Ride>,
    mut pickup_latitude: i32,
    mut pickup_longitude: i32,
    mut dest_latitude: i32,
    mut dest_longitude: i32,
) -> sqlx::Result<i32> {
    let discount = if let Some(ride) = ride {
        dest_latitude = ride.destination_latitude;
        dest_longitude = ride.destination_longitude;
        pickup_latitude = ride.pickup_latitude;
        pickup_longitude = ride.pickup_longitude;

        // すでにクーポンが紐づいているならそれの割引額を参照
        let coupon: Option<Coupon> = sqlx::query_as("SELECT * FROM coupons WHERE used_by = ?")
            .bind(&ride.id)
            .fetch_optional(&mut *tx)
            .await?;
        coupon.map(|c| c.discount).unwrap_or(0)
    } else {
        // 初回利用クーポンを最優先で使う
        let coupon: Option<Coupon> = sqlx::query_as(
            "SELECT * FROM coupons WHERE user_id = ? AND code = 'CP_NEW2024' AND used_by IS NULL",
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(coupon) = coupon {
            coupon.discount
        } else {
            // 無いなら他のクーポンを付与された順番に使う
            let coupon: Option<Coupon> = sqlx::query_as("SELECT * FROM coupons WHERE user_id = ? AND used_by IS NULL ORDER BY created_at LIMIT 1")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
            coupon.map(|c| c.discount).unwrap_or(0)
        }
    };

    let metered_fare = crate::FARE_PER_DISTANCE
        * crate::calculate_distance(
            pickup_latitude,
            pickup_longitude,
            dest_latitude,
            dest_longitude,
        );
    let discounted_metered_fare = std::cmp::max(metered_fare - discount, 0);

    Ok(crate::INITIAL_FARE + discounted_metered_fare)
}
