package benchrun

import (
	"errors"
	"os"
	"strconv"
)

// GetTargetAddress returns a target address given by a supervisor ($ISUXBENCH_TARGET environment variable.)
func GetTargetAddress() string {
	return os.Getenv("ISUXBENCH_TARGET")
}

// GetAllAddresses returns all addresses given by a supervisor ($ISUXBENCH_ALL_ADDRESSES environment variable.)
func GetAllAddresses() string {
	return os.Getenv("ISUXBENCH_ALL_ADDRESSES")
}

// GetReportFD returns a file descriptor for reporting results given by a supervisor ($ISUXBENCH_REPORT_FD environment variable.)
func GetReportFD() (int, error) {
	defer os.Unsetenv("ISUXBENCH_REPORT_FD")

	strFD := os.Getenv("ISUXBENCH_REPORT_FD")
	if len(strFD) == 0 {
		return 0, errors.New("ISUXBENCH_REPORT_FD is not set")
	}

	fd, err := strconv.Atoi(strFD)
	if err != nil {
		return 0, err
	}

	return fd, nil
}
