package payment

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestNewPayment(t *testing.T) {
	p := NewPayment("test")
	assert.Equal(t, "test", p.IdempotencyKey)
	assert.Equal(t, StatusInitial, p.Status.Type)
	assert.False(t, p.locked.Load())
}
