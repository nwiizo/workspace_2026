package scenario

import (
	"context"
	"log/slog"
	"time"

	"github.com/isucon/isucon14/bench/benchmarker/webapp"
)

func PostValidation(ctx context.Context, target string, addr string) error {
	clientConfig := webapp.ClientConfig{
		TargetBaseURL:         target,
		TargetAddr:            addr,
		ClientIdleConnTimeout: 10 * time.Second,
	}

	if err := validateInitialData(ctx, clientConfig); err != nil {
		slog.String("初期データのバリデーションに失敗", err.Error())
		return err
	}

	return nil
}
