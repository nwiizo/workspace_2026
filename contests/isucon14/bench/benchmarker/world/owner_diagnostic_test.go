package world

import (
	"testing"
	"time"

	"github.com/guregu/null/v5"
)

func TestOwnerDistanceDiagnosticSnapshotMatchesValidationWatermark(t *testing.T) {
	location := ChairLocation{Initial: C(0, 0)}
	location.PlaceTo(&LocationEntry{
		Coord:      C(0, 0),
		ServerTime: null.TimeFrom(time.UnixMilli(1)),
	})
	location.MoveTo(&LocationEntry{
		Coord:      C(2, 0),
		ServerTime: null.TimeFrom(time.UnixMilli(3)),
	})
	location.MoveTo(&LocationEntry{
		Coord:      C(4, 0),
		ServerTime: null.TimeFrom(time.UnixMilli(2)),
	})

	snapshot := location.ownerDistanceDiagnosticSnapshot(time.UnixMilli(2), true)

	// TotalTravelDistanceUntil stops at the first future timestamp. This
	// intentionally differs from sorting the entries by ServerTime.
	if snapshot.DistanceAtWatermark != location.TotalTravelDistanceUntil(time.UnixMilli(2)) {
		t.Fatalf(
			"diagnostic distance=%d validation distance=%d",
			snapshot.DistanceAtWatermark,
			location.TotalTravelDistanceUntil(time.UnixMilli(2)),
		)
	}
	if snapshot.CurrentDistance != 4 {
		t.Fatalf("current distance=%d", snapshot.CurrentDistance)
	}
	if len(snapshot.FullHistoryEntries) != 3 {
		t.Fatalf("full history entries=%d", len(snapshot.FullHistoryEntries))
	}
	if snapshot.FullHistoryEntries[2].TravelDistance != 4 {
		t.Fatalf(
			"last travel distance=%d",
			snapshot.FullHistoryEntries[2].TravelDistance,
		)
	}
}
