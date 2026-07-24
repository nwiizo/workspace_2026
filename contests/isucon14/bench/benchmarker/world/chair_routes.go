package world

import (
	"slices"
	"sync"
	"time"

	"github.com/guregu/null/v5"
)

type ChairLocation struct {
	// Initial 初期位置
	Initial Coordinate

	current             *LocationEntry
	history             []*LocationEntry
	totalTravelDistance int
	dirty               bool

	mu sync.RWMutex
}

type LocationEntry struct {
	Coord      Coordinate
	Time       int64
	ServerTime null.Time
}

func (r *ChairLocation) Current() Coordinate {
	r.mu.RLock()
	defer r.mu.RUnlock()
	if r.current == nil {
		return r.Initial
	}
	return r.current.Coord
}

func (r *ChairLocation) LastMovedAt() (time.Time, bool) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	if r.current == nil {
		return time.Time{}, false
	}
	for _, entry := range slices.Backward(r.history) {
		if entry.ServerTime.Valid {
			return entry.ServerTime.Time, true
		}
	}
	return time.Time{}, false
}

func (r *ChairLocation) TotalTravelDistance() int {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.totalTravelDistance
}

func (r *ChairLocation) TotalTravelDistanceUntil(until time.Time) int {
	r.mu.RLock()
	defer r.mu.RUnlock()

	sum := 0
	prev := r.Initial
	for _, entry := range r.history {
		if entry.ServerTime.Valid {
			if entry.ServerTime.Time.After(until) {
				break
			} else {
				sum += prev.DistanceTo(entry.Coord)
				prev = entry.Coord
			}
		}
	}
	return sum
}

func (r *ChairLocation) ResetDirtyFlag() {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.dirty = false
}

func (r *ChairLocation) Dirty() bool {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.dirty
}

// PlaceTo 椅子をlocationに配置する。前回の位置との距離差を総移動距離には加算しない
func (r *ChairLocation) PlaceTo(location *LocationEntry) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.history = append(r.history, location)
	r.current = location
	r.dirty = true
}

// MoveTo 椅子をlocationに移動させ、総移動距離を加算する
func (r *ChairLocation) MoveTo(location *LocationEntry) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.history = append(r.history, location)
	r.totalTravelDistance += r.current.Coord.DistanceTo(location.Coord)
	r.current = location
	r.dirty = true
}

func (r *ChairLocation) SetServerTime(serverTime time.Time) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.current.ServerTime = null.TimeFrom(serverTime)
}

type GetPeriodsByCoordResultEntry struct {
	Since time.Time
	Until null.Time
}

func (r *ChairLocation) GetPeriodsByCoord(c Coordinate) []GetPeriodsByCoordResultEntry {
	r.mu.RLock()
	defer r.mu.RUnlock()
	var (
		result  []GetPeriodsByCoordResultEntry
		current *GetPeriodsByCoordResultEntry
	)
	for _, entry := range r.history {
		if current != nil && entry.ServerTime.Valid {
			current.Until = entry.ServerTime
			result = append(result, *current)
			current = nil
		}
		if entry.Coord.Equals(c) && entry.ServerTime.Valid {
			current = &GetPeriodsByCoordResultEntry{
				Since: entry.ServerTime.Time,
			}
		}
	}
	if current != nil {
		result = append(result, *current)
	}
	return result
}

func (r *ChairLocation) GetCoordByTime(t time.Time) Coordinate {
	r.mu.RLock()
	defer r.mu.RUnlock()

	for _, entry := range slices.Backward(r.history) {
		if entry.ServerTime.Valid && !entry.ServerTime.Time.After(t) {
			return entry.Coord
		}
	}
	return r.Initial
}

func (r *ChairLocation) GetLocationEntryByTime(t time.Time) *LocationEntry {
	r.mu.RLock()
	defer r.mu.RUnlock()

	for _, entry := range slices.Backward(r.history) {
		if entry.ServerTime.Valid && !entry.ServerTime.Time.After(t) {
			return entry
		}
	}
	return nil
}
