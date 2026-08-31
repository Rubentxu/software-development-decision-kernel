# Go Testing Reference

Patterns for the **Go** half of the pyramid. Covers stdlib `testing`, table-driven tests, sub-tests, fuzzing, and benchmarks. For deep Go testing patterns, see the `go-testing` skill (table-driven, teatest, golden files).

## Unit tests (the foundation)

```go
// user.go
func ValidateEmail(s string) error { ... }

// user_test.go
func TestValidateEmail(t *testing.T) {
    tests := []struct {
        name    string
        input   string
        wantErr bool
    }{
        {"empty", "", true},
        {"missing @", "abc", true},
        {"valid", "a@b.c", false},
        {"unicode local", "ünîcödé@b.c", false},
    }
    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            err := ValidateEmail(tt.input)
            if (err != nil) != tt.wantErr {
                t.Errorf("ValidateEmail(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
            }
        })
    }
}
```

**Rules**:
- Table-driven tests with `t.Run(tt.name, ...)` — see `go-testing` skill.
- `t.TempDir()` for filesystem tests; never rely on a real home directory.
- Test behavior, not implementation.
- Integration tests should be skippable with `testing.Short()`.

## Sub-tests and parallelism

```go
func TestParallel(t *testing.T) {
    tests := []struct{ name string; ... }{...}
    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            t.Parallel()  // run sub-tests in parallel
            ...
        })
    }
}
```

## HTTP testing (stdlib `httptest`)

```go
func TestHandler(t *testing.T) {
    req := httptest.NewRequest("POST", "/users", strings.NewReader(`{"email":"a@b.c"}`))
    req.Header.Set("Content-Type", "application/json")
    rr := httptest.NewRecorder()
    handler := http.HandlerFunc(createUser)
    handler.ServeHTTP(rr, req)

    if rr.Code != http.StatusCreated {
        t.Errorf("status = %d, want 201", rr.Code)
    }
}
```

For a real server, use `httptest.NewServer` and a `http.Client`.

## Database integration

- `database/sql` with `sqlmock` for unit-level DB code.
- `dockertest` / `testcontainers-go` for real DB in CI.
- `testfixtures` for loading YAML/JSON fixtures.
- `goose` / `migrate` / `golang-migrate` for migrations (run as test setup).

## Mocks and fakes

- Stdlib `testing` has no mocking; use:
  - `testify/mock` — struct-based mocks.
  - `gomock` + `mockgen` — interface mocks generated from code.
  - `counterfeiter` — typed fakes (preferred over gomock for readability).
  - Hand-written fakes — usually the cleanest option for repos / gateways.

**Rule**: prefer hand-written fakes for repositories. The fake becomes a runnable spec of the contract.

## Fuzzing (Go 1.18+)

```go
func FuzzParseJSON(f *testing.F) {
    f.Add(`{"name":"a@b.c"}`)
    f.Fuzz(func(t *testing.T, input string) {
        var v MyStruct
        if err := json.Unmarshal([]byte(input), &v); err != nil {
            return  // invalid JSON is expected
        }
        out, err := json.Marshal(v)
        if err != nil {
            t.Fatalf("marshal: %v", err)
        }
        var back MyStruct
        if err := json.Unmarshal(out, &back); err != nil {
            t.Fatalf("unmarshal: %v", err)
        }
        if !reflect.DeepEqual(v, back) {
            t.Errorf("roundtrip mismatch")
        }
    })
}
```

Run with `go test -fuzz=FuzzParseJSON -fuzztime=30s`.

## Race detection

```bash
go test -race ./...
```

Always run in CI. `-race` is cheap and catches data races that unit tests miss.

## Coverage

```bash
go test -cover ./...
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out -o coverage.html
go test -covermode=atomic -coverpkg=./... ./...
```

## Benchmarks

```go
func BenchmarkParse(b *testing.B) {
    data := loadSample()
    b.ResetTimer()
    for i := 0; i < b.N; i++ {
        _, _ = Parse(data)
    }
}
```

Run with `go test -bench=. -benchmem ./...`. Compare with `benchstat` before/after.

## Anti-patterns

- Hitting a real network in tests; use `httptest.NewServer` or `httpmock` (e.g., `go-httpmock`).
- `time.Sleep(...)` in tests; inject a `Clock` interface.
- Module-level shared state (singleton clients, global DB).
- Mocking the SUT instead of its dependencies.
- Tests that depend on test order.

## References to load

- `go-testing` — Comprehensive Go testing patterns (table-driven, teatest, golden, command mocking).
- `diagnose` — For hard flakiness or test-timeout bugs.
- `work-unit-commits` — Keep tests in the same commit as the behavior.
