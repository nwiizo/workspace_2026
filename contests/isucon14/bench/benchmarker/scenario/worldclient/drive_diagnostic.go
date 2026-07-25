package worldclient

import (
	"encoding/json"
	"os"
	"sync"
)

const (
	traceRideBuckets = uint64(32)
	fnvOffsetBasis   = uint64(0xcbf29ce484222325)
	fnvPrime         = uint64(0x100000001b3)
)

var (
	driveDiagnosticOnce    sync.Once
	driveDiagnosticEnabled bool
	driveDiagnosticLines   chan driveDiagnosticMessage
)

type driveDiagnosticMessage struct {
	line  []byte
	flush chan struct{}
}

func driveDiagnosticsEnabled() bool {
	driveDiagnosticOnce.Do(func() {
		driveDiagnosticEnabled = os.Getenv("ISUCON_DIAGNOSTIC") == "1"
		if driveDiagnosticEnabled {
			driveDiagnosticLines = make(chan driveDiagnosticMessage, 16_384)
			go func() {
				for message := range driveDiagnosticLines {
					if message.flush != nil {
						close(message.flush)
						continue
					}
					_, _ = os.Stdout.Write(message.line)
				}
			}()
		}
	})
	return driveDiagnosticEnabled
}

func rideBucket(rideID string) uint64 {
	hash := fnvOffsetBasis
	for _, value := range []byte(rideID) {
		hash = (hash ^ uint64(value)) * fnvPrime
	}
	return hash % traceRideBuckets
}

func shouldTraceRide(rideID string) bool {
	return driveDiagnosticsEnabled() && rideBucket(rideID) == 0
}

func emitDriveDiagnostic(prefix string, sample any) {
	encoded, err := json.Marshal(sample)
	if err != nil {
		return
	}
	line := make([]byte, 0, len(prefix)+len(encoded)+2)
	line = append(line, prefix...)
	line = append(line, ' ')
	line = append(line, encoded...)
	line = append(line, '\n')
	driveDiagnosticLines <- driveDiagnosticMessage{line: line}
}

// FlushDriveDiagnostics waits until every line enqueued before this barrier has
// been written. Validation calls it after the load phase and before process exit.
func FlushDriveDiagnostics() {
	if !driveDiagnosticsEnabled() {
		return
	}
	flushed := make(chan struct{})
	driveDiagnosticLines <- driveDiagnosticMessage{flush: flushed}
	<-flushed
}
