package concurrent

import "iter"

// TryIter ブロッキング無しでchから値が取り出せるだけ取り出すイテレーターを返します
func TryIter[T any](ch <-chan T) iter.Seq[T] {
	return func(yield func(T) bool) {
		for {
			select {
			case v := <-ch:
				if !yield(v) {
					return
				}
			default:
				return
			}
		}
	}
}

// TrySend ブロッキング無しのchへの送信を試みます
func TrySend[T any](ch chan<- T, v T) bool {
	select {
	case ch <- v:
		return true
	default:
		return false
	}
}
