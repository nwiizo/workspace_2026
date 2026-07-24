package world

import "testing"

func TestDesiredChairNum(t *testing.T) {
	n := 0
	for i := 0; i < 30000; i++ {
		num := desiredChairNum(i * 100)
		if num > n {
			n = num
			t.Logf("num: %d (original: %d), sales: %d", num, i*100/15000, i*100)
		}
	}
}
