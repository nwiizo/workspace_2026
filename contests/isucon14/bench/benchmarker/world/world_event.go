package world

type Event interface {
	isWorldEvent()
}

type unimplementedEvent struct{}

func (*unimplementedEvent) isWorldEvent() {}

type EventRequestCompleted struct {
	Request *Request

	unimplementedEvent
}

type EventUserActivated struct {
	User *User

	unimplementedEvent
}

type EventUserLeave struct {
	User *User

	unimplementedEvent
}

type EventSoftError struct {
	Error error

	unimplementedEvent
}
