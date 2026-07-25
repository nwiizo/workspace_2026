package worldclient

import (
	"fmt"
	"testing"
)

func TestRideBucketMatchesServerSelection(t *testing.T) {
	tests := map[string]uint64{
		"01JTEST0000000000000000000": 12,
		"01JTEST0000000000000000001": 31,
	}
	for rideID, expected := range tests {
		if actual := rideBucket(rideID); actual != expected {
			t.Fatalf("rideBucket(%q) = %d, want %d", rideID, actual, expected)
		}
	}
}

func TestRideBucketDistributesSequentialIDs(t *testing.T) {
	selected := 0
	for number := 0; number < 3_200; number++ {
		if rideBucket(fmt.Sprintf("ride-%04d", number)) == 0 {
			selected++
		}
	}
	if selected < 70 || selected > 130 {
		t.Fatalf("selected = %d, want 70..130", selected)
	}
}
