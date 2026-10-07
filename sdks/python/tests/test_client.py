"""Tests for the Overseer Python client."""

import pytest
from httpx import HTTPStatusError

from overseer import OverseerClient
from overseer.models import Conversation, AgentInfo, ToolInfo


@pytest.mark.asyncio
async def test_client_init() -> None:
    """Client initialization works with default params."""
    client = OverseerClient()
    assert client.base_url == "http://localhost:8080"
    assert client.token is None
    await client.close()


@pytest.mark.asyncio
async def test_client_init_with_token() -> None:
    """Client initialization with token."""
    client = OverseerClient(token="test-token")
    assert client.token == "test-token"
    assert client.auth_header == {"Authorization": "Bearer test-token"}
    await client.close()


@pytest.mark.asyncio
async def test_client_init_with_custom_url() -> None:
    """Client initialization with custom base URL."""
    client = OverseerClient("https://overseer.example.com:9090")
    assert client.base_url == "https://overseer.example.com:9090"
    await client.close()


@pytest.mark.asyncio
async def test_connection_refused() -> None:
    """Should raise on connection refused."""
    client = OverseerClient("http://localhost:1")
    with pytest.raises(Exception):
        await client.conversations.list()
    await client.close()


@pytest.mark.asyncio
async def test_model_creation() -> None:
    """Pydantic models can be created from dicts."""
    conv = Conversation(id=1, title="Test", created_at="2024-01-01T00:00:00")
    assert conv.id == 1
    assert conv.title == "Test"

    agent = AgentInfo(id=1, name="test-agent", description="Test", domain="test", tools=["read_file"], instructions="Assist", model="local", version="1", source_files=[], response_format="json")
    assert agent.name == "test-agent"

    tool = ToolInfo(id=1, name="test-tool", description="Test", language="python", code="", safety_level="low", version="1")
    assert tool.name == "test-tool"


@pytest.mark.asyncio
async def test_token_setter() -> None:
    """Token can be set after initialization."""
    client = OverseerClient()
    assert client.token is None
    client.token = "new-token"
    assert client.token == "new-token"
    await client.close()


@pytest.mark.asyncio
async def test_async_context_manager() -> None:
    """Async context manager works."""
    async with OverseerClient() as client:
        assert client is not None
    # Client should be closed after context exit
