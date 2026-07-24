package cmd

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"time"

	"github.com/isucon/isucandar"
	"github.com/spf13/cobra"
	"go.opentelemetry.io/otel"

	"github.com/isucon/isucon14/bench/benchmarker/metrics"
	"github.com/isucon/isucon14/bench/benchmarker/scenario"
	"github.com/isucon/isucon14/bench/benchrun"
	"github.com/isucon/isucon14/bench/benchrun/gen/isuxportal/resources"
)

var (
	// ベンチマークターゲット(URL)
	targetURL string
	// ベンチマークターゲット(ip:port)
	targetAddr string
	// ペイメントサーバのURL
	paymentURL string
	// ペイメントサーバーがバインドするポート番号
	paymentBindPort int
	// 負荷走行秒数 (0のときは負荷走行を実行せずprepareのみ実行する)
	loadTimeoutSeconds int64
	// 再起動後のデータ保持チェックモードかどうか
	postValidationMode bool
	// エラーが発生した際に非0のexit codeを返すかどうか
	failOnError bool
	// メトリクスを出力するかどうか
	exportMetrics bool
	// 静的ファイルのチェックをスキップするかどうか
	skipStaticFileSanityCheck bool
)

var jst = time.FixedZone("Asia/Tokyo", 9*60*60)

var runCmd = &cobra.Command{
	Use:   "run",
	Short: "Run a benchmark",
	PreRunE: func(cmd *cobra.Command, args []string) error {
		// supervisorで起動された場合は、targetを上書きする
		if benchrun.GetTargetAddress() != "" {
			targetURL = "https://xiv.isucon.net"
			targetAddr = fmt.Sprintf("%s:%d", benchrun.GetTargetAddress(), 443)
		}

		return nil
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		slog.SetDefault(slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{
			AddSource: true,
			Level:     slog.LevelDebug,
			ReplaceAttr: func(_ []string, a slog.Attr) slog.Attr {
				if a.Key == "time" && a.Value.Kind() == slog.KindTime {
					return slog.String(a.Key, a.Value.Time().In(jst).Format("15:04:05.000"))
				}
				return a
			},
		})))
		contestantLogger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
			ReplaceAttr: func(_ []string, a slog.Attr) slog.Attr {
				if a.Key == "time" && a.Value.Kind() == slog.KindTime {
					return slog.String(a.Key, a.Value.Time().In(jst).Format("15:04:05.000"))
				}
				return a
			},
		}))

		var reporter benchrun.Reporter
		if fd, err := benchrun.GetReportFD(); err != nil {
			reporter = &benchrun.NullReporter{}
		} else {
			if reporter, err = benchrun.NewFDReporter(fd); err != nil {
				return fmt.Errorf("failed to create reporter: %w", err)
			}
		}

		exporter, err := metrics.Setup(!exportMetrics)
		if err != nil {
			return fmt.Errorf("failed to create meter: %w", err)
		}
		defer exporter.Shutdown(context.Background())

		slog.Debug("target", slog.String("targetURL", targetURL), slog.String("targetAddr", targetAddr), slog.String("benchrun.GetTargetAddress()", benchrun.GetTargetAddress()), slog.String("paymentURL", paymentURL))

		if postValidationMode {
			contestantLogger.Info("post validationを実行します")
			passed := false
			if err := scenario.PostValidation(cmd.Context(), targetURL, targetAddr); err != nil {
				contestantLogger.Error(err.Error())
			} else {
				passed = true
			}
			if err := reporter.Report(&resources.BenchmarkResult{
				Finished:       true,
				Passed:         passed,
				Score:          0,
				ScoreBreakdown: &resources.BenchmarkResult_ScoreBreakdown{Raw: 0, Deduction: 0},
				Execution:      &resources.BenchmarkResult_Execution{Reason: "実行終了"},
			}); err != nil {
				slog.Error(err.Error())
			}
			contestantLogger.Info("post validationが終了しました")
			return nil
		}

		s := scenario.NewScenario(targetURL, targetAddr, paymentURL, paymentBindPort, contestantLogger, reporter, otel.Meter("isucon14_benchmarker"), loadTimeoutSeconds == 0, skipStaticFileSanityCheck)

		b, err := isucandar.NewBenchmark(
			isucandar.WithoutPanicRecover(),
			isucandar.WithLoadTimeout(time.Duration(loadTimeoutSeconds)*time.Second),
		)
		if err != nil {
			return fmt.Errorf("failed to create benchmark: %w", err)
		}
		b.AddScenario(s)

		var errors []error
		if loadTimeoutSeconds == 0 {
			contestantLogger.Info("prepareのみを実行します")
			result := b.Start(context.Background())
			errors = result.Errors.All()
			contestantLogger.Info("prepareが終了しました",
				slog.Any("errors", errors),
			)
		} else {
			contestantLogger.Info("負荷走行を開始します")
			result := b.Start(context.Background())
			errors = result.Errors.All()
			contestantLogger.Info("負荷走行が終了しました")
			if len(errors) > 0 {
				slog.Error("発生したエラー", slog.Any("errors", errors))
			}
		}

		if failOnError && len(errors) > 0 {
			os.Exit(1)
		}
		return nil
	},
}

func init() {
	runCmd.Flags().StringVar(&targetURL, "target", "http://localhost:8080", "benchmark target url")
	runCmd.Flags().StringVar(&targetAddr, "addr", "", "benchmark target ip:port")
	runCmd.Flags().StringVar(&paymentURL, "payment-url", "http://localhost:12345", "payment server URL")
	runCmd.Flags().IntVar(&paymentBindPort, "payment-bind-port", 12345, "payment server bind port")
	runCmd.Flags().Int64VarP(&loadTimeoutSeconds, "load-timeout", "t", 60, "load timeout in seconds (When this value is 0, load does not run and only prepare is run)")
	runCmd.Flags().BoolVar(&failOnError, "fail-on-error", false, "fail on error")
	runCmd.Flags().BoolVar(&postValidationMode, "only-post-validation", false, "post validation mode")
	runCmd.Flags().BoolVar(&exportMetrics, "metrics", false, "whether to output metrics")
	runCmd.Flags().BoolVarP(&skipStaticFileSanityCheck, "skip-static-sanity-check", "s", false, "skip static file validation")
	rootCmd.AddCommand(runCmd)
}
