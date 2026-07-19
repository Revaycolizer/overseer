#!/usr/bin/env python3
"""
Overseer REST API Client — single-file reference implementation.

Usage:
    python scripts/client.py --help

Examples:
    export OVESEER_TOKEN=$(python scripts/client.py login --username admin --password secret)
    python scripts/client.py conversations list
    python scripts/client.py agents list
    python scripts/client.py orchestrate --conversation-id 1 --query "Hello"
"""

import argparse
import json
import os
import sys
from typing import Any, Optional
from urllib.parse import urlencode

import httpx


class OverseerClient:
    """Minimal HTTP client for the Overseer REST API."""

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        token: Optional[str] = None,
        timeout: float = 30.0,
    ):
        self.base_url = base_url.rstrip("/")
        self.token = token or os.environ.get("OVESEER_TOKEN")
        self.http = httpx.Client(base_url=self.base_url, timeout=timeout, headers=self._headers())

    def _headers(self) -> dict[str, str]:
        headers = {"User-Agent": "overseer-client.py/0.1"}
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        return headers

    def _get(self, path: str, params: Optional[dict] = None) -> Any:
        r = self.http.get(path, params=params)
        r.raise_for_status()
        return r.json() if r.text else None

    def _post(self, path: str, body: Any = None) -> Any:
        r = self.http.post(path, json=body or {})
        r.raise_for_status()
        return r.json() if r.text else None

    # ── Auth ───────────────────────────────────────────────────────────

    def login(self, username: str, password: str) -> str:
        """Login and return a bearer token."""
        result = self._post("/v1/auth/login", {"username": username, "password": password})
        self.token = result["token"]
        return result["token"]

    def register(self, username: str, display_name: str, password: str) -> dict:
        return self._post("/v1/auth/register", {
            "username": username, "display_name": display_name, "password": password,
        })

    def me(self) -> dict:
        return self._get("/v1/auth/me")

    # ── Conversations ──────────────────────────────────────────────────

    def list_conversations(self, limit: int = 50) -> list[dict]:
        return self._get("/api/conversations", {"limit": limit})

    def create_conversation(self, title: str) -> int:
        return self._post("/api/conversations", {"title": title})

    def delete_conversation(self, conv_id: int) -> None:
        self._post("/api/conversations/delete", {"id": conv_id})

    def search_messages(self, query: str) -> list[dict]:
        return self._get("/api/conversations/search", {"q": query})

    # ── Messages ───────────────────────────────────────────────────────

    def list_messages(self, conversation_id: int) -> list[dict]:
        return self._get("/api/messages", {"conversation_id": conversation_id})

    def send_message(self, conversation_id: int, role: str, content: str, hide_evidence: bool = False) -> int:
        return self._post("/api/messages", {
            "conversation_id": conversation_id,
            "role": role,
            "content": content,
            "hide_evidence": hide_evidence,
        })

    # ── Agents ─────────────────────────────────────────────────────────

    def list_agents(self) -> list[dict]:
        return self._get("/api/agents")

    def create_agent(self, name: str, description: str, domain: str) -> int:
        return self._post("/api/agents", {"name": name, "description": description, "domain": domain})

    # ── Tools ──────────────────────────────────────────────────────────

    def list_tools(self) -> list[dict]:
        return self._get("/api/tools")

    def create_tool(self, name: str, description: str, language: str) -> int:
        return self._post("/api/tools", {"name": name, "description": description, "language": language})

    # ── Orchestrate ────────────────────────────────────────────────────

    def orchestrate(self, conversation_id: int, query: str) -> str:
        return self._post("/api/orchestrate", {"conversation_id": conversation_id, "query": query})

    # ── Health ─────────────────────────────────────────────────────────

    def health(self) -> dict:
        return self._get("/health")

    # ── System ─────────────────────────────────────────────────────────

    def system_load(self) -> dict:
        return self._get("/api/system/load")

    def available_models(self) -> list[str]:
        return self._get("/api/models/available")


# ── CLI ──────────────────────────────────────────────────────────────

def make_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="overseer", description="Overseer API client")
    p.add_argument("--base-url", default="http://localhost:8080", help="Server base URL")
    p.add_argument("--token", help="Bearer token (or OVESEER_TOKEN env var)")
    p.add_argument("--json", action="store_true", help="Output raw JSON")

    sub = p.add_subparsers(dest="command", required=True)

    # login
    login_p = sub.add_parser("login", help="Authenticate and print token")
    login_p.add_argument("--username", required=True)
    login_p.add_argument("--password", required=True)

    # conversations
    conv_p = sub.add_parser("conversations", help="Conversation operations")
    conv_sub = conv_p.add_subparsers(dest="action", required=True)
    conv_sub.add_parser("list", help="List conversations")
    create_conv = conv_sub.add_parser("create", help="Create conversation")
    create_conv.add_argument("--title", required=True)
    delete_conv = conv_sub.add_parser("delete", help="Delete conversation")
    delete_conv.add_argument("--id", type=int, required=True)

    # messages
    msg_p = sub.add_parser("messages", help="Message operations")
    msg_sub = msg_p.add_subparsers(dest="action", required=True)
    list_msg = msg_sub.add_parser("list", help="List messages")
    list_msg.add_argument("--conversation-id", type=int, required=True)
    send_msg = msg_sub.add_parser("send", help="Send a message")
    send_msg.add_argument("--conversation-id", type=int, required=True)
    send_msg.add_argument("--role", default="user")
    send_msg.add_argument("--content", required=True)

    # agents
    sub.add_parser("agents", help="List agents")

    # tools
    sub.add_parser("tools", help="List tools")

    # supervise
    sub.add_parser("health", help="Check server health")

    # orchestrate
    orch_p = sub.add_parser("orchestrate", help="Send a query to the agent system")
    orch_p.add_argument("--conversation-id", type=int, required=True)
    orch_p.add_argument("--query", required=True)

    # models
    sub.add_parser("models", help="List available models")

    return p


def main() -> int:
    parser = make_parser()
    args = parser.parse_args()

    client = OverseerClient(base_url=args.base_url, token=args.token)

    try:
        if args.command == "login":
            token = client.login(args.username, args.password)
            print(token)
            return 0

        if args.command == "conversations":
            if args.action == "list":
                result = client.list_conversations()
            elif args.action == "create":
                result = client.create_conversation(args.title)
            elif args.action == "delete":
                result = client.delete_conversation(args.id)
                print(f"Deleted conversation {args.id}")
                return 0
            else:
                print(f"Unknown action: {args.action}", file=sys.stderr)
                return 1

        elif args.command == "messages":
            if args.action == "list":
                result = client.list_messages(args.conversation_id)
            elif args.action == "send":
                result = client.send_message(args.conversation_id, args.role, args.content)
            else:
                print(f"Unknown action: {args.action}", file=sys.stderr)
                return 1

        elif args.command == "agents":
            result = client.list_agents()

        elif args.command == "tools":
            result = client.list_tools()

        elif args.command == "health":
            result = client.health()

        elif args.command == "orchestrate":
            result = client.orchestrate(args.conversation_id, args.query)

        elif args.command == "models":
            result = client.available_models()

        else:
            print(f"Unknown command: {args.command}", file=sys.stderr)
            return 1

        if args.json:
            json.dump(result, sys.stdout, indent=2, default=str)
        elif isinstance(result, str):
            print(result)
        elif isinstance(result, (list, dict)):
            json.dump(result, sys.stdout, indent=2, default=str)
        else:
            print(result)
        print()
        return 0

    except httpx.HTTPStatusError as e:
        print(f"Error {e.response.status_code}: {e.response.text}", file=sys.stderr)
        return 1
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
