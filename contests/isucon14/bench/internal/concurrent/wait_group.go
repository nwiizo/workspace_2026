package concurrent

import (
	"sync"
	"sync/atomic"
)

// WaitChan wg.Waitが完了するまでブロックするチャネルを作成する
func WaitChan(wg *sync.WaitGroup) <-chan struct{} {
	c := make(chan struct{})
	go func() {
		defer close(c)
		wg.Wait()
	}()
	return c
}

// WaitGroupWithCount カウンタ付きsync.WaitGroup
type WaitGroupWithCount struct {
	sync.WaitGroup
	count int64
}

func (wg *WaitGroupWithCount) Add(delta int) {
	atomic.AddInt64(&wg.count, int64(delta))
	wg.WaitGroup.Add(delta)
}

func (wg *WaitGroupWithCount) Done() {
	atomic.AddInt64(&wg.count, -1)
	wg.WaitGroup.Done()
}

// Count Doneになっていない数を返す
func (wg *WaitGroupWithCount) Count() int {
	return int(atomic.LoadInt64(&wg.count))
}
