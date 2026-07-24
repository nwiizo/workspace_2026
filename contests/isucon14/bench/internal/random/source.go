package random

import (
	"math/rand/v2"
	"sync"
)

type lockedSource struct {
	inner rand.Source
	sync.Mutex
}

func (r *lockedSource) Uint64() uint64 {
	r.Lock()
	defer r.Unlock()
	return r.inner.Uint64()
}

func NewLockedSource(src rand.Source) rand.Source {
	return &lockedSource{
		inner: src,
	}
}

func NewLockedRand(src rand.Source) *rand.Rand {
	return rand.New(NewLockedSource(src))
}

func CreateChildSource(parent rand.Source) rand.Source {
	return rand.NewPCG(parent.Uint64(), parent.Uint64())
}

func CreateChildRand(parent rand.Source) *rand.Rand {
	return NewLockedRand(CreateChildSource(parent))
}
