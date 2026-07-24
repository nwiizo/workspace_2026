package world

import (
	"strconv"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestRegion_Contains(t *testing.T) {
	regionA := &Region{
		RegionOffsetX: 0,
		RegionOffsetY: 0,
		RegionWidth:   100,
		RegionHeight:  200,
	}
	regionB := &Region{
		RegionOffsetX: 50,
		RegionOffsetY: 100,
		RegionWidth:   100,
		RegionHeight:  200,
	}
	tests := []struct {
		region *Region
		coord  Coordinate
		expect bool
	}{
		{
			region: regionA,
			coord:  Coordinate{0, 0},
			expect: true,
		},
		{
			region: regionA,
			coord:  Coordinate{50, 100},
			expect: true,
		},
		{
			region: regionA,
			coord:  Coordinate{50, -100},
			expect: true,
		},
		{
			region: regionA,
			coord:  Coordinate{-50, 100},
			expect: true,
		},
		{
			region: regionA,
			coord:  Coordinate{-50, -100},
			expect: true,
		},
		{
			region: regionA,
			coord:  Coordinate{51, 100},
			expect: false,
		},
		{
			region: regionA,
			coord:  Coordinate{51, -100},
			expect: false,
		},
		{
			region: regionA,
			coord:  Coordinate{-50, 101},
			expect: false,
		},
		{
			region: regionA,
			coord:  Coordinate{-50, -101},
			expect: false,
		},
		{
			region: regionB,
			coord:  Coordinate{0, 0},
			expect: true,
		},
		{
			region: regionB,
			coord:  Coordinate{100, 0},
			expect: true,
		},
		{
			region: regionB,
			coord:  Coordinate{0, 200},
			expect: true,
		},
		{
			region: regionB,
			coord:  Coordinate{100, 200},
			expect: true,
		},
		{
			region: regionB,
			coord:  Coordinate{-1, 0},
			expect: false,
		},
		{
			region: regionB,
			coord:  Coordinate{101, 0},
			expect: false,
		},
		{
			region: regionB,
			coord:  Coordinate{0, 201},
			expect: false,
		},
		{
			region: regionB,
			coord:  Coordinate{0, -1},
			expect: false,
		},
	}
	for i, tt := range tests {
		t.Run(strconv.Itoa(i), func(t *testing.T) {
			assert.Equal(t, tt.expect, tt.region.Contains(tt.coord))
		})
	}
}
