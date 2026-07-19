"""Authentication module for the Overseer client."""

from typing import Optional

from .models import AuthResponse, UserProfile


class AuthManager:
    """Manages authentication tokens and user sessions."""

    def __init__(self) -> None:
        self._token: Optional[str] = None
        self._user: Optional[UserProfile] = None

    @property
    def token(self) -> Optional[str]:
        return self._token

    @token.setter
    def token(self, value: str) -> None:
        self._token = value

    @property
    def user(self) -> Optional[UserProfile]:
        return self._user

    def set_auth(self, response: AuthResponse) -> None:
        """Store token and user info from a login response."""
        self._token = response.token
        self._user = response.user

    def clear(self) -> None:
        """Clear stored auth state (logout)."""
        self._token = None
        self._user = None

    @property
    def is_authenticated(self) -> bool:
        return self._token is not None

    @property
    def auth_header(self) -> dict[str, str]:
        """Return the Authorization header dict, or empty if unauthenticated."""
        if self._token:
            return {"Authorization": f"Bearer {self._token}"}
        return {}
