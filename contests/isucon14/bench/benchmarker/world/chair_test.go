package world

import (
	"fmt"
	"math/rand/v2"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestChair_moveRandom(t *testing.T) {
	region := &Region{
		RegionOffsetX: 0,
		RegionOffsetY: 0,
		RegionWidth:   100,
		RegionHeight:  100,
	}
	c := Chair{
		Region: region,
		Model:  &ChairModel{Speed: 13},
		Rand:   rand.New(rand.NewPCG(rand.Uint64(), rand.Uint64())),
	}
	c.Location.PlaceTo(&LocationEntry{Coord: C(0, 0)})
	for i := range int64(1000) {
		prev := c.Location.Current()
		c.Location.MoveTo(&LocationEntry{Coord: c.moveRandom(), Time: i})
		assert.Equal(t, c.Model.Speed, prev.DistanceTo(c.Location.Current()), "ランダムに動く量は常にSpeedと一致しなければならない")
		assert.True(t, c.Location.Current().Within(region), "ランダムに動く範囲はリージョン内に収まっている")
	}
}

func TestChair_moveToward(t *testing.T) {
	tests := []struct {
		chair *Chair
		dest  Coordinate
	}{
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(30, 30)},
				Model:    &ChairModel{Speed: 13},
			},
			dest: C(30, 30),
		},
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(0, 0)},
				Model:    &ChairModel{Speed: 13},
			},
			dest: C(30, 30),
		},
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(0, 0)},
				Model:    &ChairModel{Speed: 13},
			},
			dest: C(-30, 30),
		},
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(0, 0)},
				Model:    &ChairModel{Speed: 13},
			},
			dest: C(30, -30),
		},
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(0, 0)},
				Model:    &ChairModel{Speed: 13},
			},
			dest: C(-30, -30),
		},
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(0, 0)},
				Model:    &ChairModel{Speed: 10},
			},
			dest: C(30, 30),
		},
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(0, 0)},
				Model:    &ChairModel{Speed: 10},
			},
			dest: C(-30, 30),
		},
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(0, 0)},
				Model:    &ChairModel{Speed: 10},
			},
			dest: C(30, -30),
		},
		{
			chair: &Chair{
				Location: ChairLocation{Initial: C(0, 0)},
				Model:    &ChairModel{Speed: 10},
			},
			dest: C(-30, -30),
		},
	}
	for _, tt := range tests {
		tt.chair.Rand = rand.New(rand.NewPCG(rand.Uint64(), rand.Uint64()))
		t.Run(fmt.Sprintf("%s->%s,speed:%d", tt.chair.Location.Initial, tt.dest, tt.chair.Model.Speed), func(t *testing.T) {
			// 初期位置から目的地までの距離
			distance := tt.chair.Location.Initial.DistanceTo(tt.dest)
			// 到着までにかかるループ数
			expectedTick := neededTime(distance, tt.chair.Model.Speed)

			t.Logf("distance: %d, expected ticks: %d", distance, expectedTick)

			for range 100 {
				tt.chair.Location.PlaceTo(&LocationEntry{Coord: tt.chair.Location.Initial, Time: -1})
				for tick := range int64(expectedTick) {
					prev := tt.chair.Location.Current()
					require.NotEqual(t, tt.dest, prev, "必要なループ数より前に到着している")

					tt.chair.Location.MoveTo(&LocationEntry{
						Coord: tt.chair.moveToward(tt.dest),
						Time:  tick,
					})
					if !tt.dest.Equals(tt.chair.Location.Current()) {
						require.Equal(t, tt.chair.Model.Speed, prev.DistanceTo(tt.chair.Location.Current()), "目的地に到着するまでの１ループあたりの移動量は常にSpeedと一致しないといけない")
					}
				}
				require.Equal(t, tt.dest, tt.chair.Location.Current(), "想定しているループ数で到着できていない")
			}
		})
	}
}
