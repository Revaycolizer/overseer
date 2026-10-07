"""One-turn streaming chat with explicit approval decisions and bounded frames."""
import asyncio
import inspect
import math
import json
from collections.abc import AsyncIterator, Awaitable, Callable
from typing import Any
from urllib.parse import urlsplit, urlunsplit

ApprovalHandler = Callable[[dict[str, Any]], bool | Awaitable[bool]]


class ChatAPI:
    def __init__(self, base_url: str, token: Callable[[], str | None]) -> None:
        self._base_url = base_url
        self._token = token

    async def stream(
        self, message: str, conversation_id: int = 0, *, model: str | None = None,
        on_approval: ApprovalHandler | None = None, timeout: float = 120.0,
    ) -> AsyncIterator[dict[str, Any]]:
        """Yield session/token/approval/done frames; reject approvals without a handler.

        Wrap in contextlib.aclosing when exiting early. Cancelling closes the socket.
        Each receive has a timeout; human approval callbacks are not timed.
        """
        if isinstance(conversation_id, bool) or not isinstance(conversation_id, int) or not -(2**63) <= conversation_id < 2**63:
            raise ValueError("conversation_id must be a signed 64-bit integer")
        if not message.strip() or len(message.encode()) > 65536:
            raise ValueError("message must contain 1–65536 bytes")
        if not math.isfinite(timeout) or timeout <= 0:
            raise ValueError("timeout must be positive")
        try:
            from websockets.asyncio.client import connect
        except ImportError as error:
            raise ImportError('Install the streaming extra: pip install "overseer-pysdk[streaming]"') from error
        parts = urlsplit(self._base_url)
        url = urlunsplit(("wss" if parts.scheme == "https" else "ws", parts.netloc, "/api/ws/chat", "", ""))
        async with connect(url, open_timeout=timeout, max_size=4 * 1024 * 1024) as socket:
            if self._token():
                await socket.send(json.dumps({"token": self._token()}))
            request = {"conversation_id": str(conversation_id), "message": message}
            if model is not None:
                request["model"] = model
            await socket.send(json.dumps(request))
            while True:
                raw = await asyncio.wait_for(socket.recv(), timeout=timeout)
                event = json.loads(raw)
                if not isinstance(event, dict) or not isinstance(event.get("type"), str):
                    raise ValueError("Invalid chat event")
                yield event
                if event["type"] == "error":
                    raise RuntimeError(event.get("error", "Chat failed"))
                if event["type"] == "done":
                    return
                if event["type"] == "approval":
                    decision = on_approval(event) if on_approval else False
                    if inspect.isawaitable(decision):
                        decision = await decision
                    await socket.send(json.dumps({"approve" if decision is True else "reject": event["id"]}))
