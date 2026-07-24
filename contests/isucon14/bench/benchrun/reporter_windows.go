//go:build windows

package benchrun

import (
	"bufio"
	"encoding/binary"
	"errors"
	"os"
	"syscall"

	"github.com/isucon/isucon14/bench/benchrun/gen/isuxportal/resources"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/timestamppb"
)

type Reporter interface {
	Report(result *resources.BenchmarkResult) error
}

// NullReporter is for testing
type NullReporter struct{}

func (rep *NullReporter) Report(result *resources.BenchmarkResult) error {
	return nil
}

type FDReporter struct {
	io *bufio.Writer
}

func (rep *FDReporter) Report(result *resources.BenchmarkResult) error {
	if result.SurveyResponse != nil && len(result.SurveyResponse.Language) > 140 {
		return errors.New("language in a given survey response is too long (max: 140)")
	}

	if result.MarkedAt == nil {
		result.MarkedAt = timestamppb.Now()
	}

	wire, err := proto.Marshal(result)
	if err != nil {
		return err
	}
	if len(wire) > 65536 {
		return errors.New("marshalled BenchmarkResult is too long (max: 65536)")
	}

	lenBuf := make([]byte, 2)
	binary.BigEndian.PutUint16(lenBuf, uint16(len(wire)))

	if _, err := rep.io.Write(lenBuf); err != nil {
		return err
	}
	if _, err := rep.io.Write(wire); err != nil {
		return err
	}
	if err := rep.io.Flush(); err != nil {
		return err
	}

	return nil
}

func NewFDReporter(fd int) (*FDReporter, error) {
	syscall.CloseOnExec(syscall.Handle(fd))
	io := os.NewFile(uintptr(fd), "ISUXBENCH_REPORT_FD")

	bufWriter := bufio.NewWriter(io)

	rep := &FDReporter{
		io: bufWriter,
	}

	return rep, nil
}
