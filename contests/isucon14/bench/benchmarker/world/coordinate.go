package world

import (
	"fmt"
	"math/rand/v2"
)

// Coordinate 座標
type Coordinate struct {
	X int
	Y int
}

func C(x, y int) Coordinate {
	return Coordinate{X: x, Y: y}
}

func (c Coordinate) String() string {
	return fmt.Sprintf("(%d,%d)", c.X, c.Y)
}

func (c Coordinate) Equals(c2 Coordinate) bool {
	return c.X == c2.X && c.Y == c2.Y
}

// DistanceTo c2までのマンハッタン距離
func (c Coordinate) DistanceTo(c2 Coordinate) int {
	return abs(c.X-c2.X) + abs(c.Y-c2.Y)
}

// Within 座標がregion内にあるかどうか
func (c Coordinate) Within(region *Region) bool {
	return region.Contains(c)
}

func (c Coordinate) MoveToward(target Coordinate, step int, rand *rand.Rand) Coordinate {
	prev := c
	to := c

	// ランダムにx, y方向で近づける
	x := rand.IntN(step + 1)
	y := step - x
	remain := 0

	switch {
	case prev.X < target.X:
		xDiff := target.X - (prev.X + x)
		if xDiff < 0 {
			// X座標で追い越すので、追い越す分をyの移動に加える
			to.X = target.X
			y += -xDiff
		} else {
			to.X += x
		}
	case prev.X > target.X:
		xDiff := (prev.X - x) - target.X
		if xDiff < 0 {
			// X座標で追い越すので、追い越す分をyの移動に加える
			to.X = target.X
			y += -xDiff
		} else {
			to.X -= x
		}
	default:
		y = step
	}

	switch {
	case prev.Y < target.Y:
		yDiff := target.Y - (prev.Y + y)
		if yDiff < 0 {
			to.Y = target.Y
			remain += -yDiff
		} else {
			to.Y += y
		}
	case prev.Y > target.Y:
		yDiff := (prev.Y - y) - target.Y
		if yDiff < 0 {
			to.Y = target.Y
			remain += -yDiff
		} else {
			to.Y -= y
		}
	default:
		remain = y
	}

	if remain > 0 {
		x = remain
		switch {
		case to.X < target.X:
			xDiff := target.X - (to.X + x)
			if xDiff < 0 {
				to.X = target.X
			} else {
				to.X += x
			}
		case to.X > target.X:
			xDiff := (to.X - x) - target.X
			if xDiff < 0 {
				to.X = target.X
			} else {
				to.X -= x
			}
		}
	}

	return to
}

func RandomCoordinate(worldX, worldY int) Coordinate {
	return C(rand.IntN(worldX), rand.IntN(worldY))
}

func RandomCoordinateWithRand(worldX, worldY int, rand *rand.Rand) Coordinate {
	return C(rand.IntN(worldX), rand.IntN(worldY))
}

func RandomCoordinateOnRegion(region *Region) Coordinate {
	return C(region.RegionOffsetX+rand.IntN(region.RegionWidth)-region.RegionWidth/2, region.RegionOffsetY+rand.IntN(region.RegionHeight)-region.RegionHeight/2)
}

func RandomCoordinateOnRegionWithRand(region *Region, rand *rand.Rand) Coordinate {
	return C(region.RegionOffsetX+rand.IntN(region.RegionWidth)-region.RegionWidth/2, region.RegionOffsetY+rand.IntN(region.RegionHeight)-region.RegionHeight/2)
}

func RandomCoordinateAwayFromHereWithRand(here Coordinate, distance int, rand *rand.Rand) Coordinate {
	// 移動量の決定
	x := rand.IntN(distance + 1)
	y := distance - x

	// 移動方向の決定
	switch rand.IntN(4) {
	case 0:
		x *= -1
	case 1:
		y *= -1
	case 2:
		x *= -1
		y *= -1
	case 3:
		break
	}
	return C(here.X+x, here.Y+y)
}

func RandomTwoCoordinateWithRand(region *Region, distance int, rand *rand.Rand) (Coordinate, Coordinate) {
	c1 := RandomCoordinateOnRegionWithRand(region, rand)

	shiftX := rand.IntN(distance + 1)
	shiftY := distance - shiftX

	if rand.IntN(2) == 0 {
		shiftX = -shiftX
	}
	if rand.IntN(2) == 0 {
		shiftY = -shiftY
	}

	c2 := C(c1.X+shiftX, c1.Y+shiftY)
	if !region.Contains(c2) {
		return RandomTwoCoordinateWithRand(region, distance, rand) // Retry
	}

	return c1, c2
}

func CalculateRandomDetourPoint(start, dest Coordinate, speed int, rand *rand.Rand) Coordinate {
	halfT := start.DistanceTo(dest) / speed / 2
	move := halfT * speed
	moveX := rand.IntN(move + 1)
	moveY := move - moveX

	if start.X == dest.X {
		moveX = move
		moveY = 0
	} else if start.Y == dest.Y {
		moveY = move
		moveX = 0
	}

	x := start.X
	y := start.Y
	switch {
	case start.X < dest.X:
		x += moveX
	case start.X > dest.X:
		x -= moveX
	}

	switch {
	case start.Y < dest.Y:
		y += moveY
	case start.Y > dest.Y:
		y -= moveY
	}

	return C(x, y)
}
