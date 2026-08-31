package probe

type Checker interface {
	Check() error
}

func Probe(_ Checker) error {
	return nil
}
