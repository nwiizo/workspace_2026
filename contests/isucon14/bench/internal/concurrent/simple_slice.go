package concurrent

import (
	"iter"
	"slices"
	"sync"
)

type SimpleSlice[V any] struct {
	s    []V
	lock sync.RWMutex
}

func NewSimpleSlice[V any]() *SimpleSlice[V] {
	return &SimpleSlice[V]{
		s: []V{},
	}
}

func (s *SimpleSlice[V]) Append(value V) {
	s.lock.Lock()
	defer s.lock.Unlock()
	s.s = append(s.s, value)
}

func (s *SimpleSlice[V]) Len() int {
	s.lock.RLock()
	defer s.lock.RUnlock()
	return len(s.s)
}

func (s *SimpleSlice[V]) Iter() iter.Seq2[int, V] {
	return func(yield func(int, V) bool) {
		s.lock.RLock()
		defer s.lock.RUnlock()
		for i, v := range s.s {
			if !yield(i, v) {
				break
			}
		}
	}
}

func (s *SimpleSlice[V]) BackwardIter() iter.Seq2[int, V] {
	return func(yield func(int, V) bool) {
		s.lock.RLock()
		defer s.lock.RUnlock()
		for i, v := range slices.Backward(s.s) {
			if !yield(i, v) {
				break
			}
		}
	}
}

func (s *SimpleSlice[V]) Values() iter.Seq[V] {
	return func(yield func(V) bool) {
		s.lock.RLock()
		defer s.lock.RUnlock()
		for _, v := range s.s {
			if !yield(v) {
				return
			}
		}
	}
}

func (s *SimpleSlice[V]) ToSlice() []V {
	return slices.Collect(s.Values())
}
