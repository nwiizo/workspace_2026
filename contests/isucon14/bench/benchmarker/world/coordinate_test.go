package world

import (
	"math/rand/v2"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestRandomCoordinateAwayFromHereWithRand(t *testing.T) {
	r := rand.New(rand.NewPCG(rand.Uint64(), rand.Uint64()))

	c := C(0, 0)
	for range 1000 {
		prev := c
		distance := r.IntN(100)
		c = RandomCoordinateAwayFromHereWithRand(c, distance, r)
		assert.Equal(t, distance, prev.DistanceTo(c), "離れる量は常にdistanceと一致しなければならない")
	}
}
