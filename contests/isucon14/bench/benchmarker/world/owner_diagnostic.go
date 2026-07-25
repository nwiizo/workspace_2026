package world

import (
	"encoding/json"
	"os"
	"sync"
	"sync/atomic"
	"time"
)

var ownerDistanceDiagnosticOutput sync.Mutex
var ownerDistanceFullHistoryCaptured atomic.Bool

type ownerDistanceHistoryEntryDiagnostic struct {
	Coordinate       Coordinate `json:"coordinate"`
	ClientTime       int64      `json:"client_time"`
	ServerTimeUnixUs *int64     `json:"server_time_unix_us"`
	TravelDistance   int        `json:"travel_distance"`
}

type ownerDistanceLocationDiagnostic struct {
	InitialCoordinate   Coordinate                            `json:"initial_coordinate"`
	DistanceAtWatermark int                                   `json:"distance_at_watermark"`
	CurrentDistance     int                                   `json:"current_distance"`
	HistoryEntries      int                                   `json:"history_entries"`
	KnownServerTimes    int                                   `json:"known_server_times"`
	UnknownServerTimes  int                                   `json:"unknown_server_times"`
	RecentEntries       []ownerDistanceHistoryEntryDiagnostic `json:"recent_entries"`
	FullHistoryEntries  []ownerDistanceHistoryEntryDiagnostic `json:"full_history_entries,omitempty"`
}

type ownerDistanceBenchmarkDiagnostic struct {
	Reason                  string                          `json:"reason"`
	OwnerID                 int                             `json:"owner_id"`
	ChairID                 string                          `json:"chair_id"`
	RequestStartedAtUnixUs  int64                           `json:"request_started_at_unix_us"`
	ResponseWatermarkUnixUs int64                           `json:"response_watermark_unix_us"`
	ResponseTotalDistance   int                             `json:"response_total_distance"`
	InitialExpectedDistance int                             `json:"initial_expected_distance"`
	InitialCurrentDistance  int                             `json:"initial_current_distance"`
	LastKnownMovedAtUnixUs  *int64                          `json:"last_known_moved_at_unix_us"`
	Location                ownerDistanceLocationDiagnostic `json:"location"`
}

func (r *ChairLocation) ownerDistanceDiagnosticSnapshot(
	until time.Time,
	captureFullHistory bool,
) ownerDistanceLocationDiagnostic {
	r.mu.RLock()
	defer r.mu.RUnlock()

	snapshot := ownerDistanceLocationDiagnostic{
		InitialCoordinate: r.Initial,
		CurrentDistance:   r.totalTravelDistance,
		HistoryEntries:    len(r.history),
	}
	watermarkPrev := r.Initial
	for _, entry := range r.history {
		if entry.ServerTime.Valid {
			if !entry.ServerTime.Time.After(until) {
				snapshot.DistanceAtWatermark += watermarkPrev.DistanceTo(entry.Coord)
				watermarkPrev = entry.Coord
			} else {
				break
			}
		}
	}

	const recentEntryLimit = 8
	start := max(0, len(r.history)-recentEntryLimit)
	snapshot.RecentEntries = make([]ownerDistanceHistoryEntryDiagnostic, 0, len(r.history)-start)
	historyPrev := r.Initial
	historyDistance := 0
	for index, entry := range r.history {
		historyDistance += historyPrev.DistanceTo(entry.Coord)
		historyPrev = entry.Coord
		var serverTimeUnixUs *int64
		if entry.ServerTime.Valid {
			snapshot.KnownServerTimes++
			value := entry.ServerTime.Time.UnixMicro()
			serverTimeUnixUs = &value
		} else {
			snapshot.UnknownServerTimes++
		}
		diagnostic := ownerDistanceHistoryEntryDiagnostic{
			Coordinate:       entry.Coord,
			ClientTime:       entry.Time,
			ServerTimeUnixUs: serverTimeUnixUs,
			TravelDistance:   historyDistance,
		}
		if captureFullHistory {
			snapshot.FullHistoryEntries = append(snapshot.FullHistoryEntries, diagnostic)
		}
		if index >= start {
			snapshot.RecentEntries = append(snapshot.RecentEntries, diagnostic)
		}
	}
	return snapshot
}

func captureFullOwnerDistanceHistory() bool {
	return ownerDistanceFullHistoryCaptured.CompareAndSwap(false, true)
}

func ownerDistanceDiagnosticsEnabled() bool {
	return os.Getenv("ISUCON_DIAGNOSTIC") == "1"
}

func emitOwnerDistanceBenchmarkDiagnostic(sample ownerDistanceBenchmarkDiagnostic) {
	encoded, err := json.Marshal(sample)
	if err != nil {
		return
	}
	line := make([]byte, 0, len(encoded)+38)
	line = append(line, "OWNER_DISTANCE_BENCHMARK_DIAGNOSTIC "...)
	line = append(line, encoded...)
	line = append(line, '\n')

	ownerDistanceDiagnosticOutput.Lock()
	defer ownerDistanceDiagnosticOutput.Unlock()
	_, _ = os.Stdout.Write(line)
}
