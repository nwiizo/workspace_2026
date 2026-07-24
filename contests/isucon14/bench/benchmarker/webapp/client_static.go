package webapp

import (
	"context"
	"fmt"
	"net/http"

	"github.com/isucon/isucon14/bench/benchrun"
)

func (c *Client) StaticGetFileHash(ctx context.Context, path string) (string, error) {
	req, err := c.agent.NewRequest(http.MethodGet, path, nil)
	if err != nil {
		return "", err
	}

	resp, err := c.agent.Do(ctx, req)
	if err != nil {
		return "", fmt.Errorf("GET %sのリクエストが失敗しました: %w", path, err)
	}
	defer closeBody(resp)

	if resp.StatusCode < 200 || resp.StatusCode >= 400 {
		return "", fmt.Errorf("GET %sへのリクエストに対して、期待されたHTTPステータスコードが確認できませんでした (expected:200～399, actual:%d)", path, resp.StatusCode)
	}

	hash, err := benchrun.GetHashFromStream(resp.Body)
	if err != nil {
		return "", fmt.Errorf("GET %sのレスポンスのボディの取得に失敗しました: %w", path, err)
	}

	return hash, nil
}
