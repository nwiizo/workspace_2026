package concurrent

import "iter"

type SimpleSet[K comparable] struct {
	m *SimpleMap[K, struct{}]
}

func NewSimpleSet[K comparable]() *SimpleSet[K] {
	return &SimpleSet[K]{m: NewSimpleMap[K, struct{}]()}
}

func (s *SimpleSet[K]) Has(key K) bool {
	_, ok := s.m.Get(key)
	return ok
}

func (s *SimpleSet[K]) Add(key K) {
	s.m.Set(key, struct{}{})
}

func (s *SimpleSet[K]) Delete(key K) {
	s.m.Delete(key)
}

func (s *SimpleSet[K]) Len() int {
	return s.m.Len()
}

func (s *SimpleSet[K]) Iter() iter.Seq[K] {
	return func(yield func(K) bool) {
		for k, _ := range s.m.Iter() {
			if !yield(k) {
				break
			}
		}
	}
}
