# Go readiness probe

`Probe` must call the supplied checker and propagate its error. It must not
report success when the dependency fails.
