package world

import (
	"fmt"
	"sync/atomic"

	"golang.org/x/exp/constraints"
)

// Interval 閉区間
type Interval[T constraints.Integer | constraints.Float] struct {
	Left  T
	Right T
}

func NewInterval[T constraints.Integer | constraints.Float](left T, right T) Interval[T] {
	return Interval[T]{
		Left:  left,
		Right: right,
	}
}

func (i *Interval[T]) String() string {
	return fmt.Sprintf("[%v,%v]", i.Left, i.Right)
}

// Include 閉区間にvが含まれるかどうか
func (i *Interval[T]) Include(v T) bool {
	return i.Left <= v && v <= i.Right
}

// abs 絶対値を取る
func abs[T constraints.Signed](x T) T {
	if x < 0 {
		return -x
	}
	return x
}

// neededTime 速さsで長さdに進むのに必要な時間(小数切り上げ)
func neededTime[T constraints.Integer](d T, s T) T {
	t := d / s
	if d%s > 0 {
		t += 1
	}
	return t
}

// ConvertHour h時間を仮想世界時間に変換する
func ConvertHour[T constraints.Integer](h T) T {
	return h * LengthOfHour
}

func UnwrapMultiError(err error) ([]error, bool) {
	if errors, ok := err.(interface{ Unwrap() []error }); ok {
		return errors.Unwrap(), true
	}
	return nil, false
}

type tickDone struct {
	f atomic.Bool
}

func (t *tickDone) DoOrSkip() (skip bool) {
	return !t.f.CompareAndSwap(false, true)
}

func (t *tickDone) Done() {
	if !t.f.CompareAndSwap(true, false) {
		panic("2重でDoneした")
	}
}
