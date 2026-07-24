package world

import (
	"math"
	"sync"
	"sync/atomic"

	"github.com/isucon/isucon14/bench/internal/concurrent"
)

type Region struct {
	Name          string
	RegionOffsetX int
	RegionOffsetY int
	RegionWidth   int
	RegionHeight  int
	// UsersDB 地域内のユーザー
	UsersDB *concurrent.SimpleMap[UserID, *User]
	// TotalEvaluation 地域内のユーザーのリクエストの平均評価の合計値
	TotalEvaluation atomic.Int32
	// activeUserNum 地域内のアクティブユーザー数
	activeUserNum     int
	activeUserNumLock sync.RWMutex
}

func NewRegion(name string, offsetX, offsetY, width, height int) *Region {
	return &Region{
		Name:          name,
		RegionOffsetX: offsetX,
		RegionOffsetY: offsetY,
		RegionWidth:   width,
		RegionHeight:  height,
		UsersDB:       concurrent.NewSimpleMap[UserID, *User](),
	}
}

// Contains Regionが座標cを含んでいるかどうか
func (r *Region) Contains(c Coordinate) bool {
	left, right := r.RangeX()
	if !(left <= c.X && c.X <= right) {
		return false
	}
	bottom, top := r.RangeY()
	return bottom <= c.Y && c.Y <= top
}

// RangeX RegionのX座標の範囲
func (r *Region) RangeX() (left, right int) {
	halfWidth := r.RegionWidth / 2
	return r.RegionOffsetX - halfWidth, r.RegionOffsetX + halfWidth
}

// RangeY RegionのY座標の範囲
func (r *Region) RangeY() (bottom, top int) {
	halfHeight := r.RegionHeight / 2
	return r.RegionOffsetY - halfHeight, r.RegionOffsetY + halfHeight
}

// UserSatisfactionScore 地域内のユーザーの満足度
func (r *Region) UserSatisfactionScore() int {
	// 地域内の全ユーザーの平均評価の平均値を地域のユーザー満足度にしている
	// (ユーザーの評価の初期値は0)
	return int(math.Round(float64(r.TotalEvaluation.Load()) / float64(r.UsersDB.Len())))
}

// ActiveUserNum Regionの現在のアクティブユーザー数
func (r *Region) ActiveUserNum() int {
	r.activeUserNumLock.RLock()
	defer r.activeUserNumLock.RUnlock()
	return r.activeUserNum
}

// AddUser Regionにユーザーを追加する
func (r *Region) AddUser(u *User) {
	r.UsersDB.Set(u.ID, u)
	if u.State != UserStateInactive {
		r.activeUserNumLock.Lock()
		defer r.activeUserNumLock.Unlock()
		r.activeUserNum++
	}
}

// UserLeave Regionからユーザーが離脱可能なら離脱させる
func (r *Region) UserLeave(u *User) bool {
	r.activeUserNumLock.Lock()
	defer r.activeUserNumLock.Unlock()
	if r.activeUserNum-1 < 10 {
		// Regionに最低10人はアクティブにする
		return false
	}
	r.activeUserNum--
	u.Deactivate()
	return true
}
