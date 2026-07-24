package world

import "log/slog"

type Context struct {
	world *World
}

func NewContext(world *World) *Context {
	return &Context{
		world: world,
	}
}

func (c *Context) CurrentTime() int64 {
	return c.world.Time
}

func (c *Context) ContestantLogger() *slog.Logger {
	return c.world.contestantLogger
}
