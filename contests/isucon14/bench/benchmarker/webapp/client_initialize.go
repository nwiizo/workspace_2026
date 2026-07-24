package webapp

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/isucon/isucon14/bench/benchmarker/webapp/api"
)

type PostInitializeResponse struct {
	Language string `json:"language"`
}

func (c *Client) PostInitialize(ctx context.Context, reqBody *api.PostInitializeReq) (*PostInitializeResponse, error) {
	reqBodyBuf, err := reqBody.MarshalJSON()
	if err != nil {
		return nil, err
	}

	req, err := c.agent.NewRequest(http.MethodPost, "/api/initialize", bytes.NewReader(reqBodyBuf))
	if err != nil {
		return nil, err
	}

	for _, modifier := range c.requestModifiers {
		modifier(req)
	}

	resp, err := c.agent.Do(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("POST /api/initialize のリクエストが失敗しました: %w", err)
	}
	defer closeBody(resp)

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("POST /api/initialize へのリクエストに対して、期待されたHTTPステータスコードが確認できませんでした (expected:%d, actual:%d)", http.StatusOK, resp.StatusCode)
	}

	var response PostInitializeResponse
	if json.NewDecoder(resp.Body).Decode(&response) != nil {
		return nil, fmt.Errorf("POST /api/initializeのJSONのdecodeに失敗しました: %w", err)
	}

	return &response, nil
}
