//go:build tools
// +build tools

package main

import (
	_ "github.com/ogen-go/ogen/cmd/ogen"
	_ "go.uber.org/mock/mockgen"
)
