package world

import (
	"errors"
	"io"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestCodeError_Is(t *testing.T) {
	assert.True(t, errors.Is(WrapCodeError(ErrorCodeFailedToSendChairCoordinate, io.ErrUnexpectedEOF), CodeError(ErrorCodeFailedToSendChairCoordinate)))

}
