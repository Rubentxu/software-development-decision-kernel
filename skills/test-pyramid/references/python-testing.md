# Python Testing Reference

Patterns for the **Python** half of the pyramid. Covers pytest, unittest, hypothesis, and Django/Flask/FastAPI.

## Tooling baseline

- **Test runner**: `pytest` (preferred) or `unittest` (stdlib).
- **Property-based**: `hypothesis` (matches `proptest` in Rust).
- **Async tests**: `pytest-asyncio`.
- **HTTP API**: `httpx` (sync + async), `starlette.testclient` for ASGI in-process.
- **DB**: `pytest-postgresql`, `pytest-mysql`, or `testcontainers-python`.
- **Mocking**: `pytest-mock` (wraps `unittest.mock`), `responses` for requests, `aioresponses` for aiohttp.
- **Benchmark**: `pytest-benchmark`, `py-spy`, `memray`.

## Unit tests (the foundation)

```python
# src/utils/format.py
def format_currency(cents: int, currency: str = "USD") -> str:
    ...

# tests/test_format.py
import pytest
from utils.format import format_currency

@pytest.mark.parametrize("cents,expected", [
    (0, "$0.00"),
    (199, "$1.99"),
    (-100, "-$1.00"),
    (1000, "$10.00"),
])
def test_format_currency(cents, expected):
    assert format_currency(cents) == expected
```

**Rules**:
- One test per scenario; `@pytest.mark.parametrize` for table-driven.
- Name tests by behavior, not by input mechanics.
- Use `tmp_path` fixture (not real `~` paths) for filesystem tests.

## Property-based tests (hypothesis)

```python
from hypothesis import given, strategies as st
from utils.format import format_currency

@given(st.integers(min_value=-10**9, max_value=10**9))
def test_format_currency_always_includes_symbol(cents):
    out = format_currency(cents)
    assert "$" in out or "€" in out  # adjust
```

Hypothesis shrinks failing cases for you. Use `@example(...)` to pin specific cases that broke.

## Async tests (pytest-asyncio)

```python
import pytest

@pytest.mark.asyncio
async def test_fetch_user_returns_user():
    repo = UserRepo()
    user = await repo.get(42)
    assert user.id == 42
```

- Mark with `@pytest.mark.asyncio` per test, or set `asyncio_mode = "auto"` in `pyproject.toml`.
- One event loop per test by default.
- For trio / curio, use `trio` / `curio` markers.

## HTTP / API integration

**FastAPI (in-process, fast):**

```python
from fastapi.testclient import TestClient
from app.main import app

def test_create_user():
    with TestClient(app) as client:
        res = client.post("/users", json={"email": "a@b.c"})
        assert res.status_code == 201
```

**Django (Django test client):**

```python
from django.test import TestCase

class UserTests(TestCase):
    def test_create_user(self):
        res = self.client.post("/users", {"email": "a@b.c"})
        self.assertEqual(res.status_code, 302)  # redirect on success
```

**Flask:**

```python
import pytest
from app import create_app

@pytest.fixture
def client():
    app = create_app()
    return app.test_client()

def test_index(client):
    res = client.get("/")
    assert res.status_code == 200
```

## Database integration

```python
@pytest.fixture
def db_session():
    # pytest-postgresql / pytest-mysql / your own
    with engine.begin() as conn:
        run_migrations(conn)
    yield Session(engine)
    with engine.begin() as conn:
        drop_all(conn)

def test_user_persists(db_session):
    user = User(email="a@b.c")
    db_session.add(user)
    db_session.commit()
    loaded = db_session.query(User).filter_by(email="a@b.c").one()
    assert loaded.id is not None
```

- For Postgres / MySQL: `testcontainers-python` is the most portable.
- For SQLite-in-memory: fast, but does not support all PG/MySQL features (transactions, JSONB, etc.).
- Use **transactional rollback per test** when possible for speed and isolation.

## Mocking

| Need | Tool |
|---|---|
| Replace a function | `mocker.patch("module.func", return_value=...)` |
| Replace a class | `mocker.patch("module.Class")` |
| Mock `requests` calls | `responses` |
| Mock `httpx` calls | `respx` |
| Mock `aiohttp` calls | `aioresponses` |
| Fake clock | `freezegun` |
| Fake env | `monkeypatch.setenv(...)` |

**Rule**: mock at the seam, not deep inside the unit. The more layers you mock, the less you test.

## Django-specific

- `pytest-django` for pytest integration.
- `pytest.mark.django_db` for DB access.
- `pytest-django` provides a `client` fixture.
- `pytest-factory-boy` for fixture factories.
- Use `TransactionTestCase` only when you need real transactions; otherwise `TestCase` is faster.

## FastAPI-specific

- `TestClient(app)` is a sync wrapper; use `httpx.AsyncClient(app=app)` for async tests.
- Override dependencies with `app.dependency_overrides[dep] = lambda: fake`.
- Test the **OpenAPI schema** is stable: `client.get("/openapi.json")` and snapshot.

## Anti-patterns

- Hitting a real DB without transactional rollback.
- Mocking the SUT (system under test) instead of its dependencies.
- `time.sleep(...)` in tests; use `freezegun` or `asyncio.sleep(0)` with mock.
- Hitting a real network (`requests.get("https://api.example.com/...")`); use `responses` / `respx`.
- Sharing a module-level `client` / `session` across tests.
- `conftest.py` autouse fixtures that depend on test order.

## References to load

- `diagnose` — For hard flakiness or test-timeout bugs.
- `work-unit-commits` — Keep tests in the same commit as the behavior.
