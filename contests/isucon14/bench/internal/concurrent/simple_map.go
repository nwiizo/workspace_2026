package concurrent

import (
	"iter"
	"slices"
	"sync"
)

// SimpleMap 読み書き時に単純なRWLockを取るマップ
type SimpleMap[K comparable, V any] struct {
	m    map[K]V
	lock sync.RWMutex
}

func NewSimpleMap[K comparable, V any]() *SimpleMap[K, V] {
	return &SimpleMap[K, V]{
		m: map[K]V{},
	}
}

func (s *SimpleMap[K, V]) Get(key K) (V, bool) {
	s.lock.RLock()
	defer s.lock.RUnlock()
	v, ok := s.m[key]
	return v, ok
}

func (s *SimpleMap[K, V]) GetOrSetDefault(key K, def func() V) (result V, set bool) {
	s.lock.Lock()
	defer s.lock.Unlock()
	v, ok := s.m[key]
	if !ok {
		v = def()
		s.m[key] = v
		set = true
	}
	return v, set
}

func (s *SimpleMap[K, V]) Set(key K, value V) {
	s.lock.Lock()
	defer s.lock.Unlock()
	s.m[key] = value
}

func (s *SimpleMap[K, V]) Delete(key K) {
	s.lock.Lock()
	defer s.lock.Unlock()
	delete(s.m, key)
}

func (s *SimpleMap[K, V]) Len() int {
	s.lock.RLock()
	defer s.lock.RUnlock()
	return len(s.m)
}

func (s *SimpleMap[K, V]) Iter() iter.Seq2[K, V] {
	return func(yield func(K, V) bool) {
		s.lock.RLock()
		defer s.lock.RUnlock()
		for k, v := range s.m {
			if !yield(k, v) {
				break
			}
		}
	}
}

func (s *SimpleMap[K, V]) Values() iter.Seq[V] {
	return func(yield func(V) bool) {
		s.lock.RLock()
		defer s.lock.RUnlock()
		for _, v := range s.m {
			if !yield(v) {
				return
			}
		}
	}
}

func (s *SimpleMap[K, V]) ToSlice() []V {
	return slices.Collect(s.Values())
}
