package webapp

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strconv"

	"github.com/isucon/isucon14/bench/benchmarker/webapp/api"
)

func (c *Client) OwnerPostRegister(ctx context.Context, reqBody *api.OwnerPostOwnersReq) (*api.OwnerPostOwnersCreated, error) {
	reqBodyBuf, err := reqBody.MarshalJSON()
	if err != nil {
		return nil, err
	}

	req, err := c.agent.NewRequest(http.MethodPost, "/api/owner/owners", bytes.NewReader(reqBodyBuf))
	if err != nil {
		return nil, err
	}

	for _, modifier := range c.requestModifiers {
		modifier(req)
	}

	resp, err := c.agent.Do(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("POST /api/owner/ownersのリクエストが失敗しました: %w", err)
	}
	defer closeBody(resp)

	if resp.StatusCode != http.StatusCreated {
		return nil, fmt.Errorf("POST /api/owner/ownersへのリクエストに対して、期待されたHTTPステータスコードが確認できませませんでした (expected:%d, actual:%d)", http.StatusCreated, resp.StatusCode)
	}

	resBody := &api.OwnerPostOwnersCreated{}
	if err := json.NewDecoder(resp.Body).Decode(resBody); err != nil {
		return nil, fmt.Errorf("POST /api/owner/ownersのJSONのdecodeに失敗しました: %w", err)
	}

	return resBody, nil
}

func (c *Client) OwnerGetSales(ctx context.Context, params *api.OwnerGetSalesParams) (*api.OwnerGetSalesOK, error) {
	q := url.Values{}
	if params.Since.IsSet() {
		q.Set("since", strconv.FormatInt(params.Since.Value, 10))
	}
	if params.Until.IsSet() {
		q.Set("until", strconv.FormatInt(params.Until.Value, 10))
	}

	req, err := c.agent.NewRequest(http.MethodGet, "/api/owner/sales?"+q.Encode(), nil)
	if err != nil {
		return nil, err
	}

	for _, modifier := range c.requestModifiers {
		modifier(req)
	}

	resp, err := c.agent.Do(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("GET /api/owner/salesのリクエストが失敗しました: %w", err)
	}
	defer closeBody(resp)

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("GET /api/owner/salesへのリクエストに対して、期待されたHTTPステータスコードが確認できませんでした (expected:%d, actual:%d)", http.StatusOK, resp.StatusCode)
	}

	resBody := &api.OwnerGetSalesOK{}
	if err := json.NewDecoder(resp.Body).Decode(resBody); err != nil {
		return nil, fmt.Errorf("GET /api/owner/salesのJSONのdecodeに失敗しました: %w", err)
	}

	return resBody, nil
}

func (c *Client) OwnerGetChairs(ctx context.Context) (*api.OwnerGetChairsOK, error) {
	req, err := c.agent.NewRequest(http.MethodGet, "/api/owner/chairs", nil)
	if err != nil {
		return nil, err
	}

	for _, modifier := range c.requestModifiers {
		modifier(req)
	}

	resp, err := c.agent.Do(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("GET /api/owner/chairsのリクエストが失敗しました: %w", err)
	}
	defer closeBody(resp)

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("GET /api/owner/chairsへのリクエストに対して、期待されたHTTPステータスコードが確認できませんでした (expected:%d, actual:%d)", http.StatusOK, resp.StatusCode)
	}

	resBody := &api.OwnerGetChairsOK{}
	if err := json.NewDecoder(resp.Body).Decode(resBody); err != nil {
		return nil, fmt.Errorf("GET /api/owner/chairsのJSONのdecodeに失敗しました: %w", err)
	}

	return resBody, nil
}
