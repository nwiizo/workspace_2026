package payment

//go:generate go run go.uber.org/mock/mockgen -typed -source=$GOFILE -package=$GOPACKAGE -destination=./mock_$GOFILE

type Verifier interface {
	Verify(p *Payment) Status
}
