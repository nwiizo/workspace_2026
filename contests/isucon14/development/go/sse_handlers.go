package main

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"time"
)

func writeSSE(w http.ResponseWriter, data interface{}) error {
	buf, err := json.Marshal(data)
	if err != nil {
		return err
	}
	_, err = w.Write([]byte("data: " + string(buf) + "\n\n"))
	if err != nil {
		return err
	}

	if f, ok := w.(http.Flusher); ok {
		f.Flush()
	}

	return nil
}

func appGetNotificationSSE(w http.ResponseWriter, r *http.Request) {
	user := r.Context().Value("user").(*User)

	// Server Sent Events
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")

	var lastRide *Ride
	var lastRideStatus string
	f := func() (respond bool, err error) {
		tx, err := db.Beginx()
		if err != nil {
			return false, err
		}
		defer tx.Rollback()

		ride := &Ride{}
		err = tx.Get(ride, `SELECT * FROM rides WHERE user_id = ? ORDER BY created_at DESC LIMIT 1`, user.ID)
		if err != nil {
			if errors.Is(err, sql.ErrNoRows) {
				return false, nil
			}
			return false, err

		}
		status, err := getLatestRideStatus(tx, ride.ID)
		if err != nil {
			return false, err

		}
		if lastRide != nil && ride.ID == lastRide.ID && status == lastRideStatus {
			return false, nil
		}

		fare, err := calculateDiscountedFare(tx, user.ID, ride, ride.PickupLatitude, ride.PickupLongitude, ride.DestinationLatitude, ride.DestinationLongitude)
		if err != nil {
			return false, err
		}

		chair := &Chair{}
		stats := appGetNotificationResponseChairStats{}
		if ride.ChairID.Valid {
			if err := tx.Get(chair, `SELECT * FROM chairs WHERE id = ?`, ride.ChairID); err != nil {
				return false, err
			}
			stats, err = getChairStats(tx, chair.ID)
			if err != nil {
				return false, err
			}
		}

		if err := writeSSE(w, &appGetNotificationResponseData{
			RideID: ride.ID,
			PickupCoordinate: Coordinate{
				Latitude:  ride.PickupLatitude,
				Longitude: ride.PickupLongitude,
			},
			DestinationCoordinate: Coordinate{
				Latitude:  ride.DestinationLatitude,
				Longitude: ride.DestinationLongitude,
			},
			Fare:   fare,
			Status: status,
			Chair: &appGetNotificationResponseChair{
				ID:    chair.ID,
				Name:  chair.Name,
				Model: chair.Model,
				Stats: stats,
			},
			CreatedAt: ride.CreatedAt.UnixMilli(),
			UpdateAt:  ride.UpdatedAt.UnixMilli(),
		}); err != nil {
			return false, err
		}
		lastRide = ride
		lastRideStatus = status

		return true, nil
	}

	// 初回送信を必ず行う
	respond, err := f()
	if err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}
	if !respond {
		if err := writeSSE(w, nil); err != nil {
			writeError(w, http.StatusInternalServerError, err)
			return
		}
	}

	for {
		select {
		case <-r.Context().Done():
			w.WriteHeader(http.StatusOK)
			return

		default:
			respond, err := f()
			if err != nil {
				writeError(w, http.StatusInternalServerError, err)
				return
			}
			if !respond {
				time.Sleep(100 * time.Millisecond)
			}
		}
	}
}

func chairGetNotificationSSE(w http.ResponseWriter, r *http.Request) {
	chair := r.Context().Value("chair").(*Chair)

	// Server Sent Events
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")

	var lastRide *Ride
	var lastRideStatus string
	f := func() (respond bool, err error) {
		found := true
		ride := &Ride{}
		tx, err := db.Beginx()
		if err != nil {
			return false, err
		}
		defer tx.Rollback()

		if _, err := tx.Exec("SELECT * FROM chairs WHERE id = ? FOR UPDATE", chair.ID); err != nil {
			return false, err
		}

		if err := tx.Get(ride, `SELECT * FROM rides WHERE chair_id = ? ORDER BY updated_at DESC LIMIT 1`, chair.ID); err != nil {
			if errors.Is(err, sql.ErrNoRows) {
				found = false
			} else {
				return false, err
			}
		}

		var status string
		if found {
			status, err = getLatestRideStatus(tx, ride.ID)
			if err != nil {
				return false, err
			}
		}

		if !found || status == "COMPLETED" {
			matched := &Ride{}
			if err := tx.Get(matched, `SELECT * FROM rides WHERE chair_id IS NULL ORDER BY created_at LIMIT 1 FOR UPDATE`); err != nil {
				if errors.Is(err, sql.ErrNoRows) {
					return false, nil
				}
				return false, err
			}

			if _, err := tx.Exec("UPDATE rides SET chair_id = ? WHERE id = ?", chair.ID, matched.ID); err != nil {
				return false, err
			}

			if !found {
				ride = matched
			}
		}

		if lastRide != nil && ride.ID == lastRide.ID && status == lastRideStatus {
			return false, nil
		}

		user := &User{}
		err = tx.Get(user, "SELECT * FROM users WHERE id = ?", ride.UserID)
		if err != nil {
			return false, err
		}

		if err := tx.Commit(); err != nil {
			return false, err
		}

		if err := writeSSE(w, &chairGetNotificationResponseData{
			RideID: ride.ID,
			User: simpleUser{
				ID:   user.ID,
				Name: fmt.Sprintf("%s %s", user.Firstname, user.Lastname),
			},
			PickupCoordinate: Coordinate{
				Latitude:  ride.PickupLatitude,
				Longitude: ride.PickupLongitude,
			},
			DestinationCoordinate: Coordinate{
				Latitude:  ride.DestinationLatitude,
				Longitude: ride.DestinationLongitude,
			},
			Status: status,
		}); err != nil {
			return false, err
		}
		lastRide = ride
		lastRideStatus = status

		return true, nil
	}

	// 初回送信を必ず行う
	respond, err := f()
	if err != nil {
		writeError(w, http.StatusInternalServerError, err)
		return
	}
	if !respond {
		if err := writeSSE(w, nil); err != nil {
			writeError(w, http.StatusInternalServerError, err)
			return
		}
	}

	for {
		select {
		case <-r.Context().Done():
			w.WriteHeader(http.StatusOK)
			return

		default:
			respond, err := f()
			if err != nil {
				writeError(w, http.StatusInternalServerError, err)
				return
			}
			if !respond {
				time.Sleep(100 * time.Millisecond)
			}
		}
	}
}
