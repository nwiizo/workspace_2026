use axum::extract::State;
use axum::http::StatusCode;

use crate::matcher_diagnostic::MatcherDiagnostic;
use crate::{AppState, Error};

const MATCHING_BATCH_SIZE: usize = 64;
const MAX_SAME_REGION_PICKUP_DISTANCE: u64 = 200;

struct BenchmarkRegion {
    min_latitude: i32,
    max_latitude: i32,
    min_longitude: i32,
    max_longitude: i32,
}

const BENCHMARK_REGIONS: [BenchmarkRegion; 2] = [
    BenchmarkRegion {
        min_latitude: -50,
        max_latitude: 50,
        min_longitude: -50,
        max_longitude: 50,
    },
    BenchmarkRegion {
        min_latitude: 250,
        max_latitude: 350,
        min_longitude: 250,
        max_longitude: 350,
    },
];

#[derive(sqlx::FromRow)]
struct PendingRide {
    id: String,
    user_id: String,
    pickup_latitude: i32,
    pickup_longitude: i32,
    created_at: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow)]
struct AvailableChair {
    id: String,
    latitude: i32,
    longitude: i32,
}

struct PlannedMatch {
    ride_index: usize,
    chair: AvailableChair,
    distance: u64,
}

fn matcher_distance(a_latitude: i32, a_longitude: i32, b_latitude: i32, b_longitude: i32) -> u64 {
    (i64::from(a_latitude) - i64::from(b_latitude)).unsigned_abs()
        + (i64::from(a_longitude) - i64::from(b_longitude)).unsigned_abs()
}

fn nearest_chair_within_region(
    pickup_latitude: i32,
    pickup_longitude: i32,
    available_chairs: &[AvailableChair],
) -> Option<(usize, u64)> {
    available_chairs
        .iter()
        .enumerate()
        .filter_map(|(chair_index, chair)| {
            let distance = matcher_distance(
                pickup_latitude,
                pickup_longitude,
                chair.latitude,
                chair.longitude,
            );
            (distance <= MAX_SAME_REGION_PICKUP_DISTANCE).then_some((chair_index, distance))
        })
        .min_by_key(|(_, distance)| *distance)
}

fn plan_matches(
    pending_rides: &[PendingRide],
    mut available_chairs: Vec<AvailableChair>,
) -> Vec<PlannedMatch> {
    let mut matches = Vec::with_capacity(MATCHING_BATCH_SIZE.min(pending_rides.len()));
    for (ride_index, ride) in pending_rides.iter().enumerate() {
        if matches.len() == MATCHING_BATCH_SIZE {
            break;
        }
        let Some((chair_index, distance)) = nearest_chair_within_region(
            ride.pickup_latitude,
            ride.pickup_longitude,
            &available_chairs,
        ) else {
            continue;
        };
        matches.push(PlannedMatch {
            ride_index,
            chair: available_chairs.swap_remove(chair_index),
            distance,
        });
    }
    matches
}

pub fn internal_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/api/internal/matching",
            axum::routing::get(internal_get_matching),
        )
        .route(
            "/api/internal/diagnostics/flush",
            axum::routing::post(internal_post_diagnostics_flush),
        )
}

#[derive(serde::Serialize)]
struct DiagnosticFlushResponse {
    dropped_lines: u64,
}

async fn internal_post_diagnostics_flush() -> Result<axum::Json<DiagnosticFlushResponse>, Error> {
    if !crate::drive_diagnostic::enabled() {
        return Err(Error::NotFound("diagnostics are disabled"));
    }
    let dropped_lines = crate::drive_diagnostic::flush().await?;
    Ok(axum::Json(DiagnosticFlushResponse { dropped_lines }))
}

// このAPIをインスタンス内から一定間隔で叩かせることで、椅子とライドをマッチングさせる
async fn internal_get_matching(
    State(AppState {
        pool,
        notification_cache,
        general_db_admission,
        ..
    }): State<AppState>,
) -> Result<StatusCode, Error> {
    let _admission_guard = general_db_admission
        .acquire("internal_get_matching", &pool)
        .await;
    let mut diagnostic = MatcherDiagnostic::sampled(&pool);
    let mut tx = pool.begin().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.pool_begin_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.terminal_phase = "pending_query";
    }

    let mut pending_rides = Vec::with_capacity(MATCHING_BATCH_SIZE * BENCHMARK_REGIONS.len());
    let mut pending_selected_by_region = [0; BENCHMARK_REGIONS.len()];
    let mut pending_batch_full = false;
    for (region_index, region) in BENCHMARK_REGIONS.iter().enumerate() {
        let mut region_pending_rides: Vec<PendingRide> = sqlx::query_as(
            r#"
SELECT id, user_id, pickup_latitude, pickup_longitude, created_at
FROM rides
WHERE chair_id IS NULL
  AND pickup_latitude BETWEEN ? AND ?
  AND pickup_longitude BETWEEN ? AND ?
ORDER BY created_at, id
LIMIT ?
FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(region.min_latitude)
        .bind(region.max_latitude)
        .bind(region.min_longitude)
        .bind(region.max_longitude)
        .bind(i64::try_from(MATCHING_BATCH_SIZE).unwrap_or(i64::MAX))
        .fetch_all(&mut *tx)
        .await?;
        pending_selected_by_region[region_index] = region_pending_rides.len();
        pending_batch_full |= region_pending_rides.len() == MATCHING_BATCH_SIZE;
        pending_rides.append(&mut region_pending_rides);
    }
    pending_rides.sort_unstable_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.pending_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.pending_selected = pending_rides.len();
        diagnostic.sample.pending_selected_by_region = pending_selected_by_region;
        diagnostic.sample.pending_batch_full = pending_batch_full;
        if let Some(oldest) = pending_rides.first() {
            diagnostic.observe_oldest_pending(&oldest.id, oldest.created_at);
        }
    }

    if pending_rides.is_empty() {
        if let Some(diagnostic) = &mut diagnostic {
            diagnostic.sample.terminal_phase = "commit";
        }
        tx.commit().await?;
        if let Some(mut diagnostic) = diagnostic {
            diagnostic.sample.commit_us = Some(diagnostic.elapsed_since_checkpoint_us());
            diagnostic.emit_success();
        }
        return Ok(StatusCode::NO_CONTENT);
    }

    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.terminal_phase = "available_query";
    }
    let mut available_chairs = Vec::with_capacity(MATCHING_BATCH_SIZE * BENCHMARK_REGIONS.len());
    let mut available_selected_by_region = [0; BENCHMARK_REGIONS.len()];
    let mut available_batch_full = false;
    for (region_index, region) in BENCHMARK_REGIONS.iter().enumerate() {
        let mut region_available_chairs: Vec<AvailableChair> = sqlx::query_as(
            r#"
SELECT chairs.id,
       chair_current_locations.latitude,
       chair_current_locations.longitude
FROM chairs
INNER JOIN chair_current_locations
        ON chair_current_locations.chair_id = chairs.id
WHERE chairs.is_active = TRUE
  AND NOT EXISTS (
      SELECT 1
      FROM rides
      WHERE rides.chair_id = chairs.id
        AND (
            SELECT COUNT(ride_statuses.chair_sent_at)
            FROM ride_statuses
            WHERE ride_statuses.ride_id = rides.id
        ) <> 6
  )
  AND chair_current_locations.latitude BETWEEN ? AND ?
  AND chair_current_locations.longitude BETWEEN ? AND ?
ORDER BY chairs.id
LIMIT ?
FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(region.min_latitude)
        .bind(region.max_latitude)
        .bind(region.min_longitude)
        .bind(region.max_longitude)
        .bind(i64::try_from(MATCHING_BATCH_SIZE).unwrap_or(i64::MAX))
        .fetch_all(&mut *tx)
        .await?;
        available_selected_by_region[region_index] = region_available_chairs.len();
        available_batch_full |= region_available_chairs.len() == MATCHING_BATCH_SIZE;
        available_chairs.append(&mut region_available_chairs);
    }
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.available_query_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.available_selected = available_chairs.len();
        diagnostic.sample.available_selected_by_region = available_selected_by_region;
        diagnostic.sample.available_batch_full = available_batch_full;
        diagnostic.sample.terminal_phase = "matching_update";
    }

    let pending_selected = pending_rides.len();
    let mut matched_notifications = Vec::with_capacity(pending_rides.len());
    let planned_matches = plan_matches(&pending_rides, available_chairs);
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.matching_attempted = planned_matches.len();
    }
    for planned_match in planned_matches {
        let ride = &pending_rides[planned_match.ride_index];
        let result = sqlx::query("UPDATE rides SET chair_id = ? WHERE id = ? AND chair_id IS NULL")
            .bind(&planned_match.chair.id)
            .bind(&ride.id)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 1 {
            if let Some(diagnostic) = &mut diagnostic {
                diagnostic.observe_match_distance(planned_match.distance);
            }
            matched_notifications.push((ride.user_id.clone(), planned_match.chair.id));
        }
    }
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.matching_update_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.matched = matched_notifications.len();
        diagnostic.sample.unmatched_in_batch =
            pending_selected.saturating_sub(matched_notifications.len());
        diagnostic.sample.terminal_phase = "commit";
    }

    tx.commit().await?;
    if let Some(diagnostic) = &mut diagnostic {
        diagnostic.sample.commit_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.sample.terminal_phase = "cache_invalidation";
    }
    for (user_id, chair_id) in matched_notifications {
        notification_cache.invalidate_app(&user_id);
        notification_cache.invalidate_chair(&chair_id);
    }
    if let Some(mut diagnostic) = diagnostic {
        diagnostic.sample.cache_invalidation_us = Some(diagnostic.elapsed_since_checkpoint_us());
        diagnostic.emit_success();
    }

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::{
        matcher_distance, nearest_chair_within_region, plan_matches, AvailableChair, PendingRide,
        MATCHING_BATCH_SIZE, MAX_SAME_REGION_PICKUP_DISTANCE,
    };

    fn chair(id: &str, latitude: i32, longitude: i32) -> AvailableChair {
        AvailableChair {
            id: id.to_owned(),
            latitude,
            longitude,
        }
    }

    fn ride(id: &str, latitude: i32, longitude: i32, sequence: u32) -> PendingRide {
        PendingRide {
            id: id.to_owned(),
            user_id: format!("user-{id}"),
            pickup_latitude: latitude,
            pickup_longitude: longitude,
            created_at: chrono::DateTime::from_timestamp(i64::from(sequence), 0)
                .expect("valid test timestamp")
                .naive_utc(),
        }
    }

    #[test]
    fn nearest_chair_stays_within_the_benchmark_region() {
        let chairs = [
            chair("far-region", 250, 250),
            chair("same-region-farther", 40, 40),
            chair("same-region-nearest", 11, 10),
        ];

        let selected = nearest_chair_within_region(10, 10, &chairs);

        assert_eq!(selected, Some((2, 1)));
    }

    #[test]
    fn nearest_chair_rejects_only_candidates_from_another_region() {
        let chairs = [chair("other-region", 250, 250)];

        let selected = nearest_chair_within_region(0, 0, &chairs);

        assert_eq!(selected, None);
    }

    #[test]
    fn nearest_chair_keeps_the_distance_boundary_explicit() {
        let chairs = [
            chair(
                "at-boundary",
                i32::try_from(MAX_SAME_REGION_PICKUP_DISTANCE).expect("threshold fits i32"),
                0,
            ),
            chair(
                "outside-boundary",
                i32::try_from(MAX_SAME_REGION_PICKUP_DISTANCE + 1).expect("threshold fits i32"),
                0,
            ),
        ];

        let selected = nearest_chair_within_region(0, 0, &chairs);

        assert_eq!(selected, Some((0, MAX_SAME_REGION_PICKUP_DISTANCE)));
    }

    #[test]
    fn matcher_distance_handles_the_full_i32_coordinate_range() {
        assert_eq!(
            matcher_distance(i32::MIN, i32::MIN, i32::MAX, i32::MAX),
            8_589_934_590
        );
    }

    #[test]
    fn batch_plan_reaches_a_later_region_after_sixty_four_unmatchable_rides() {
        let mut rides = (0..MATCHING_BATCH_SIZE)
            .map(|index| ride(&format!("blocked-{index:02}"), 0, 0, index as u32))
            .collect::<Vec<_>>();
        rides.push(ride("later-region", 300, 300, MATCHING_BATCH_SIZE as u32));

        let matches = plan_matches(&rides, vec![chair("later-chair", 300, 300)]);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].ride_index, MATCHING_BATCH_SIZE);
        assert_eq!(matches[0].chair.id, "later-chair");
    }

    #[test]
    fn batch_plan_never_reuses_a_chair() {
        let rides = [ride("first", 0, 0, 0), ride("second", 1, 1, 1)];

        let matches = plan_matches(&rides, vec![chair("only-chair", 0, 0)]);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].chair.id, "only-chair");
    }
}
