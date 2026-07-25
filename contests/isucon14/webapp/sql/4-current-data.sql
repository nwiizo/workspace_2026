INSERT INTO chair_current_locations (
  chair_id,
  location_id,
  latitude,
  longitude,
  created_at
)
SELECT chair_id,
       id,
       latitude,
       longitude,
       created_at
FROM (
  SELECT chair_id,
         id,
         latitude,
         longitude,
         created_at,
         ROW_NUMBER() OVER (
           PARTITION BY chair_id
           ORDER BY created_at DESC, id DESC
         ) AS row_rank
  FROM chair_locations
) AS ranked_locations
WHERE row_rank = 1;

INSERT INTO chair_stats (
  chair_id,
  total_rides_count,
  total_evaluation_sum
)
SELECT chair_id,
       COUNT(*)        AS total_rides_count,
       SUM(evaluation) AS total_evaluation_sum
FROM (
  SELECT rides.id,
         rides.chair_id,
         rides.evaluation
  FROM rides
  INNER JOIN ride_statuses ON ride_statuses.ride_id = rides.id
  WHERE rides.chair_id IS NOT NULL
    AND rides.evaluation IS NOT NULL
  GROUP BY rides.id, rides.chair_id, rides.evaluation
  HAVING SUM(ride_statuses.status = 'ARRIVED') > 0
     AND SUM(ride_statuses.status = 'CARRYING') > 0
     AND SUM(ride_statuses.status = 'COMPLETED') > 0
) AS completed_rides
GROUP BY chair_id;
