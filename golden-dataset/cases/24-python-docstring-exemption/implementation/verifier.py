from abc import ABC, abstractmethod


class Backend(ABC):
    """Authentication backend contract."""

    @abstractmethod
    def verify(self, token: str) -> bool:
        """Return whether a token is valid."""
        ...


class TokenVerifier:
    def __init__(self, backend: Backend) -> None:
        self.backend = backend

    def verify(self, token: str) -> bool:
        return True
