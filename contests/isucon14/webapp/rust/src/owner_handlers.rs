use std::collections::{HashMap, HashSet};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use chrono::{DateTime, NaiveDate, Utc};

use crate::models::{Chair, Owner, Ride};
use crate::{AppState, Error};

const OWNER_DISTANCE_VISIBILITY_LAG_MILLISECONDS: i64 = 1_000;
const OWNER_DISTANCE_MAX_STALENESS_MILLISECONDS: i64 = 3_000;

pub fn owner_routes(app_state: AppState) -> axum::Router<AppState> {
    let routes =
        axum::Router::new().route("/api/owner/owners", axum::routing::post(owner_post_owners));

    let authed_routes = axum::Router::new()
        .route("/api/owner/sales", axum::routing::get(owner_get_sales))
        .route("/api/owner/chairs", axum::routing::get(owner_get_chairs))
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            crate::middlewares::owner_auth_middleware,
        ));

    routes.merge(authed_routes)
}

#[derive(Debug, serde::Deserialize)]
struct OwnerPostOwnersRequest {
    name: String,
}

#[derive(Debug, serde::Serialize)]
struct OwnerPostOwnersResponse {
    id: String,
    chair_register_token: String,
}

async fn owner_post_owners(
    State(AppState { pool, .. }): State<AppState>,
    jar: CookieJar,
    axum::Json(req): axum::Json<OwnerPostOwnersRequest>,
) -> Result<(CookieJar, (StatusCode, axum::Json<OwnerPostOwnersResponse>)), Error> {
    let owner_id = ulid::Ulid::new().to_string();
    let access_token = crate::secure_random_str(32);
    let chair_register_token = crate::secure_random_str(32);

    sqlx::query(
        "INSERT INTO owners (id, name, access_token, chair_register_token) VALUES (?, ?, ?, ?)",
    )
    .bind(&owner_id)
    .bind(req.name)
    .bind(&access_token)
    .bind(&chair_register_token)
    .execute(&pool)
    .await?;

    let jar = jar.add(Cookie::build(("owner_session", access_token)).path("/"));

    Ok((
        jar,
        (
            StatusCode::CREATED,
            axum::Json(OwnerPostOwnersResponse {
                id: owner_id,
                chair_register_token,
            }),
        ),
    ))
}

#[derive(Debug, serde::Serialize)]
struct ChairSales {
    id: String,
    name: String,
    sales: i32,
}

#[derive(Debug, serde::Serialize)]
struct ModelSales {
    model: String,
    sales: i32,
}

#[derive(Debug, serde::Serialize)]
struct OwnerGetSalesResponse {
    total_sales: i32,
    chairs: Vec<ChairSales>,
    models: Vec<ModelSales>,
}

#[derive(Debug, serde::Deserialize)]
struct GetOwnerSalesQuery {
    since: Option<i64>,
    until: Option<i64>,
}

async fn owner_get_sales(
    State(AppState {
        pool,
        active_ride_evaluations,
        ..
    }): State<AppState>,
    axum::Extension(owner): axum::Extension<Owner>,
    Query(query): Query<GetOwnerSalesQuery>,
) -> Result<axum::Json<OwnerGetSalesResponse>, Error> {
    let since = if let Some(since) = query.since {
        DateTime::from_timestamp_millis(since).unwrap()
    } else {
        DateTime::from_timestamp_millis(0).unwrap()
    };
    let until = if let Some(until) = query.until {
        DateTime::from_timestamp_millis(until).unwrap()
    } else {
        DateTime::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(9999, 12, 31)
                .unwrap()
                .and_hms_opt(23, 59, 59)
                .unwrap(),
            Utc,
        )
    };

    // An evaluation can commit while this endpoint is constructing its
    // snapshot, before its response has reached the benchmark client. Track
    // only ride IDs whose response delivery overlaps this request. A fixed
    // time-based exclusion could hide a ride already included by the client's
    // lower-bound snapshot and make sales too small.
    let evaluation_snapshot = active_ride_evaluations.snapshot();
    let mut tx = pool.begin().await?;

    let chairs: Vec<Chair> = sqlx::query_as("SELECT * FROM chairs WHERE owner_id = ?")
        .bind(&owner.id)
        .fetch_all(&mut *tx)
        .await?;

    let mut rides_by_chair = Vec::with_capacity(chairs.len());
    for chair in chairs {
        let reqs: Vec<Ride> = sqlx::query_as("SELECT rides.* FROM rides JOIN ride_statuses ON rides.id = ride_statuses.ride_id WHERE chair_id = ? AND status = 'COMPLETED' AND updated_at BETWEEN ? AND ? + INTERVAL 999 MICROSECOND")
            .bind(&chair.id)
            .bind(since)
            .bind(until)
            .fetch_all(&mut *tx)
            .await?;
        let ride_sales = reqs
            .into_iter()
            .map(|ride| {
                let sale = calculate_sale(&ride);
                (ride.id, sale)
            })
            .collect::<Vec<_>>();
        rides_by_chair.push((chair, ride_sales));
    }

    let overlapping_ride_ids = active_ride_evaluations.ride_ids_overlapping(evaluation_snapshot);
    let mut res = OwnerGetSalesResponse {
        total_sales: 0,
        chairs: Vec::with_capacity(rides_by_chair.len()),
        models: Vec::new(),
    };
    let mut model_sales_by_model = HashMap::new();

    for (chair, ride_sales) in rides_by_chair {
        let sales = sum_visible_sales(&ride_sales, &overlapping_ride_ids);
        res.total_sales += sales;

        res.chairs.push(ChairSales {
            id: chair.id,
            name: chair.name,
            sales,
        });

        *model_sales_by_model.entry(chair.model).or_insert(0) += sales;
    }

    for (model, sales) in model_sales_by_model {
        res.models.push(ModelSales { model, sales });
    }

    Ok(axum::Json(res))
}

fn sum_visible_sales(ride_sales: &[(String, i32)], excluded_ride_ids: &HashSet<String>) -> i32 {
    ride_sales
        .iter()
        .filter_map(|(ride_id, sale)| (!excluded_ride_ids.contains(ride_id)).then_some(sale))
        .sum()
}

fn calculate_sale(ride: &crate::models::Ride) -> i32 {
    crate::calculate_fare(
        ride.pickup_latitude,
        ride.pickup_longitude,
        ride.destination_latitude,
        ride.destination_longitude,
    )
}

#[cfg(test)]
mod tests {
    use super::{should_suppress_owner_distance_timestamp, sum_visible_sales};
    use chrono::{DateTime, Utc};
    use std::collections::HashSet;

    fn datetime_from_micros(timestamp_micros: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_micros(timestamp_micros).expect("valid test timestamp")
    }

    #[test]
    fn owner_sales_excludes_only_overlapping_ride_ids() {
        let ride_sales = vec![
            ("completed-before".to_owned(), 700),
            ("overlapping".to_owned(), 900),
        ];
        let excluded_ride_ids = HashSet::from(["overlapping".to_owned()]);

        assert_eq!(sum_visible_sales(&ride_sales, &excluded_ride_ids), 700);
    }

    #[test]
    fn owner_distance_hides_a_stale_timestamp_while_a_recent_row_is_unstable() {
        let stable = datetime_from_micros(1_000_000);
        let latest = datetime_from_micros(5_500_000);
        let snapshot = datetime_from_micros(5_000_000);
        let freshness_boundary = datetime_from_micros(2_000_000);

        assert!(should_suppress_owner_distance_timestamp(
            Some(&stable),
            Some(&latest),
            &snapshot,
            &freshness_boundary,
        ));
    }

    #[test]
    fn owner_distance_keeps_a_fresh_stable_timestamp() {
        let stable = datetime_from_micros(4_000_000);
        let latest = datetime_from_micros(5_500_000);
        let snapshot = datetime_from_micros(5_000_000);
        let freshness_boundary = datetime_from_micros(2_000_000);

        assert!(!should_suppress_owner_distance_timestamp(
            Some(&stable),
            Some(&latest),
            &snapshot,
            &freshness_boundary,
        ));
    }

    #[test]
    fn owner_distance_keeps_an_old_timestamp_without_a_newer_unstable_row() {
        let stable = datetime_from_micros(1_000_000);
        let latest = datetime_from_micros(4_500_000);
        let snapshot = datetime_from_micros(5_000_000);
        let freshness_boundary = datetime_from_micros(2_000_000);

        assert!(!should_suppress_owner_distance_timestamp(
            Some(&stable),
            Some(&latest),
            &snapshot,
            &freshness_boundary,
        ));
    }

    #[test]
    fn owner_distance_uses_microseconds_after_the_snapshot_boundary() {
        let stable = datetime_from_micros(1_000_000);
        let latest = datetime_from_micros(5_000_050);
        let snapshot = datetime_from_micros(5_000_000);
        let freshness_boundary = datetime_from_micros(2_000_000);

        assert!(should_suppress_owner_distance_timestamp(
            Some(&stable),
            Some(&latest),
            &snapshot,
            &freshness_boundary,
        ));
    }

    #[test]
    fn owner_distance_uses_microseconds_before_the_freshness_boundary() {
        let stable = datetime_from_micros(1_999_999);
        let latest = datetime_from_micros(5_000_050);
        let snapshot = datetime_from_micros(5_000_000);
        let freshness_boundary = datetime_from_micros(2_000_000);

        assert!(should_suppress_owner_distance_timestamp(
            Some(&stable),
            Some(&latest),
            &snapshot,
            &freshness_boundary,
        ));
    }

    #[test]
    fn owner_distance_keeps_exact_boundary_values() {
        let stable = datetime_from_micros(2_000_000);
        let latest = datetime_from_micros(5_000_000);
        let snapshot = datetime_from_micros(5_000_000);
        let freshness_boundary = datetime_from_micros(2_000_000);

        assert!(!should_suppress_owner_distance_timestamp(
            Some(&stable),
            Some(&latest),
            &snapshot,
            &freshness_boundary,
        ));
    }
}

/// MySQL で COUNT()、SUM() 等を使って DECIMAL 型の値になったものを i64 に変換するための構造体。
#[derive(Debug)]
struct MysqlDecimal(i64);
impl sqlx::Decode<'_, sqlx::MySql> for MysqlDecimal {
    fn decode(
        value: sqlx::mysql::MySqlValueRef,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use sqlx::{Type as _, ValueRef as _};

        let type_info = value.type_info();
        if i64::compatible(&type_info) {
            i64::decode(value).map(Self)
        } else if u64::compatible(&type_info) {
            let n = u64::decode(value)?.try_into()?;
            Ok(Self(n))
        } else if sqlx::types::Decimal::compatible(&type_info) {
            use num_traits::ToPrimitive as _;
            let n = sqlx::types::Decimal::decode(value)?
                .to_i64()
                .expect("failed to convert DECIMAL type to i64");
            Ok(Self(n))
        } else {
            panic!("MysqlDecimal is used with unknown type: {type_info:?}");
        }
    }
}
impl sqlx::Type<sqlx::MySql> for MysqlDecimal {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        i64::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        i64::compatible(ty) || u64::compatible(ty) || sqlx::types::Decimal::compatible(ty)
    }
}
impl From<MysqlDecimal> for i64 {
    fn from(value: MysqlDecimal) -> Self {
        value.0
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ChairWithDetail {
    id: String,
    name: String,
    model: String,
    is_active: bool,
    created_at: DateTime<Utc>,
    total_distance: MysqlDecimal,
    total_distance_updated_at: Option<DateTime<Utc>>,
    latest_location_created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, serde::Serialize)]
struct OwnerGetChairResponse {
    chairs: Vec<OwnerGetChairResponseChair>,
}

#[derive(Debug, serde::Serialize)]
struct OwnerGetChairResponseChair {
    id: String,
    name: String,
    model: String,
    active: bool,
    registered_at: i64,
    total_distance: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_distance_updated_at: Option<i64>,
}

fn should_suppress_owner_distance_timestamp(
    stable_updated_at: Option<&DateTime<Utc>>,
    latest_location_created_at: Option<&DateTime<Utc>>,
    distance_snapshot_at: &DateTime<Utc>,
    freshness_boundary: &DateTime<Utc>,
) -> bool {
    matches!(
        (stable_updated_at, latest_location_created_at),
        (Some(stable), Some(latest))
            if stable < freshness_boundary && latest > distance_snapshot_at
    )
}

async fn owner_get_chairs(
    State(AppState { pool, .. }): State<AppState>,
    axum::Extension(owner): axum::Extension<Owner>,
) -> Result<axum::Json<OwnerGetChairResponse>, Error> {
    // A coordinate row becomes visible at COMMIT slightly before the chair
    // client receives recorded_at. Returning that row to an owner in this gap
    // exposes a distance watermark that the client cannot identify yet.
    // ISUCON14 explicitly allows this field to lag by up to three seconds, so
    // use one stable watermark for both the sum and updated_at.
    let request_started_at = Utc::now();
    let distance_snapshot_at = request_started_at
        - chrono::Duration::milliseconds(OWNER_DISTANCE_VISIBILITY_LAG_MILLISECONDS);
    let freshness_boundary = request_started_at
        - chrono::Duration::milliseconds(OWNER_DISTANCE_MAX_STALENESS_MILLISECONDS);
    let chairs: Vec<ChairWithDetail> = sqlx::query_as(r#"SELECT chairs.id,
       chairs.name,
       chairs.model,
       chairs.is_active,
       chairs.created_at,
       IFNULL(total_distance, 0) AS total_distance,
       total_distance_updated_at,
       chair_current_locations.created_at AS latest_location_created_at
FROM chairs
       LEFT JOIN chair_current_locations
              ON chair_current_locations.chair_id = chairs.id
       LEFT JOIN (SELECT chair_id,
                          SUM(IFNULL(distance, 0)) AS total_distance,
                          MAX(created_at)          AS total_distance_updated_at
                   FROM (SELECT chair_locations.chair_id,
                                chair_locations.created_at,
                                ABS(chair_locations.latitude - LAG(chair_locations.latitude)
                                  OVER (PARTITION BY chair_locations.chair_id ORDER BY chair_locations.created_at)) +
                                ABS(chair_locations.longitude - LAG(chair_locations.longitude)
                                  OVER (PARTITION BY chair_locations.chair_id ORDER BY chair_locations.created_at)) AS distance
                         FROM chair_locations
                         INNER JOIN chairs AS owner_chairs
                                 ON owner_chairs.id = chair_locations.chair_id
                         WHERE owner_chairs.owner_id = ?
                           AND chair_locations.created_at <= ?) tmp
                   GROUP BY chair_id) distance_table ON distance_table.chair_id = chairs.id
WHERE chairs.owner_id = ?
    "#)
    .bind(&owner.id)
    .bind(distance_snapshot_at.naive_utc())
    .bind(&owner.id)
    .fetch_all(&pool)
    .await?;

    Ok(axum::Json(OwnerGetChairResponse {
        chairs: chairs
            .into_iter()
            .map(|chair| {
                let suppress_unstable_timestamp = should_suppress_owner_distance_timestamp(
                    chair.total_distance_updated_at.as_ref(),
                    chair.latest_location_created_at.as_ref(),
                    &distance_snapshot_at,
                    &freshness_boundary,
                );

                OwnerGetChairResponseChair {
                    id: chair.id,
                    name: chair.name,
                    model: chair.model,
                    active: chair.is_active,
                    registered_at: chair.created_at.timestamp_millis(),
                    total_distance: chair.total_distance.0,
                    total_distance_updated_at: if suppress_unstable_timestamp {
                        None
                    } else {
                        chair
                            .total_distance_updated_at
                            .map(|updated_at| updated_at.timestamp_millis())
                    },
                }
            })
            .collect(),
    }))
}
