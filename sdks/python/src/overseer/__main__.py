"""CLI tool for Overseer — `overseer --help`."""

import asyncio
import json
import os
from typing import Any, Optional

import click
from rich.console import Console
from rich.json import JSON
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

from .client import OverseerClient

console = Console()
error_console = Console(stderr=True)


def _make_client(token: Optional[str] = None, base_url: str = "http://localhost:8080") -> OverseerClient:
    token = token or os.environ.get("OVERSEER_API_TOKEN")
    return OverseerClient(base_url, token=token)


def _print_result(data: Any, title: Optional[str] = None) -> None:
    """Pretty-print API results using rich."""
    if isinstance(data, list):
        if not data:
            console.print("[dim]No results.[/dim]")
            return
        table = Table(title=title)
        for key in data[0].keys():
            table.add_column(key, style="cyan")
        for item in data:
            table.add_row(*[str(v) for v in item.values()])
        console.print(table)
    elif isinstance(data, dict):
        if title:
            console.print(Panel(json.dumps(data, indent=2, default=str), title=title))
        else:
            console.print(JSON(json.dumps(data, default=str)))
    elif isinstance(data, str):
        console.print(data)
    else:
        console.print(str(data))


# ── CLI Group ─────────────────────────────────────────────────────────

@click.group()
@click.option("--base-url", default="http://localhost:8080", help="Server base URL", envvar="OVERSEER_BASE_URL")
@click.option("--token", help="Bearer token", envvar="OVERSEER_API_TOKEN")
@click.option("--json", "json_output", is_flag=True, help="Raw JSON output")
@click.pass_context
def cli(ctx: click.Context, base_url: str, token: Optional[str], json_output: bool) -> None:
    """Overseer AI orchestration platform CLI."""
    ctx.ensure_object(dict)
    ctx.obj["BASE_URL"] = base_url
    ctx.obj["TOKEN"] = token
    ctx.obj["JSON"] = json_output


# ── Auth ──────────────────────────────────────────────────────────────

@cli.command()
@click.option("--username", required=True, prompt=True)
@click.option("--password", required=True, prompt=True, hide_input=True)
@click.pass_context
def login(ctx: click.Context, username: str, password: str) -> None:
    """Authenticate and print token."""
    async def run() -> None:
        client = _make_client(base_url=ctx.obj["BASE_URL"])
        token = await client.login(username, password)
        if ctx.obj["JSON"]:
            click.echo(json.dumps({"token": token}))
        else:
            console.print(f"[green]✓[/green] Logged in as [bold]{username}[/bold]")
            console.print(f"Token: [yellow]{token[:20]}...[/yellow]")
            console.print("\nSet it in your environment:")
            console.print(f"  export OVERSEER_API_TOKEN={token}")
    asyncio.run(run())


# ── Conversations ─────────────────────────────────────────────────────

@cli.group()
def conversations() -> None:
    """Conversation operations."""


@conversations.command("list")
@click.pass_context
def list_conversations(ctx: click.Context) -> None:
    """List all conversations."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        result = await client.conversations.list()
        if ctx.obj["JSON"]:
            click.echo(json.dumps([r.model_dump(mode="json") for r in result], default=str))
        else:
            _print_result([r.model_dump(mode="json") for r in result], "Conversations")
    asyncio.run(run())


@conversations.command("create")
@click.argument("title")
@click.pass_context
def create_conversation(ctx: click.Context, title: str) -> None:
    """Create a new conversation."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        conv_id = await client.conversations.create(title)
        if ctx.obj["JSON"]:
            click.echo(json.dumps({"id": conv_id}))
        else:
            console.print(f"[green]✓[/green] Created conversation [bold]#{conv_id}[/bold]: {title}")
    asyncio.run(run())


@conversations.command("delete")
@click.argument("conv_id", type=int)
@click.pass_context
def delete_conversation(ctx: click.Context, conv_id: int) -> None:
    """Delete a conversation."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        await client.conversations.delete(conv_id)
        console.print(f"[green]✓[/green] Deleted conversation #{conv_id}")
    asyncio.run(run())


# ── Agents ────────────────────────────────────────────────────────────

@cli.command()
@click.pass_context
def agents(ctx: click.Context) -> None:
    """List all agents."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        result = await client.agents.list()
        if ctx.obj["JSON"]:
            click.echo(json.dumps([r.model_dump(mode="json") for r in result], default=str))
        else:
            _print_result([r.model_dump(mode="json") for r in result], "Agents")
    asyncio.run(run())


# ── Tools ─────────────────────────────────────────────────────────────

@cli.command()
@click.pass_context
def tools(ctx: click.Context) -> None:
    """List all tools."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        result = await client.tools.list()
        if ctx.obj["JSON"]:
            click.echo(json.dumps([r.model_dump(mode="json") for r in result], default=str))
        else:
            _print_result([r.model_dump(mode="json") for r in result], "Tools")
    asyncio.run(run())


# ── Workflows ─────────────────────────────────────────────────────────

@cli.command()
@click.pass_context
def workflows(ctx: click.Context) -> None:
    """List all workflows."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        result = await client.workflows.list()
        if ctx.obj["JSON"]:
            click.echo(json.dumps([r.model_dump(mode="json") for r in result], default=str))
        else:
            _print_result([r.model_dump(mode="json") for r in result], "Workflows")
    asyncio.run(run())


# ── Health ────────────────────────────────────────────────────────────

@cli.command()
@click.pass_context
def health(ctx: click.Context) -> None:
    """Check server health."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        h = await client.system.health()
        if ctx.obj["JSON"]:
            click.echo(h.model_dump_json(indent=2))
        else:
            status_color = "green" if h.status == "healthy" else "red"
            console.print(Panel(
                f"Status: [bold {status_color}]{h.status}[/bold {status_color}]\n"
                f"Version: [cyan]{h.version}[/cyan]\n"
                f"Uptime: {h.uptime_secs}s\n"
                f"DB: {'✅' if h.db_connected else '❌'}\n"
                f"Ollama: {'✅' if h.ollama_reachable else '❌'}",
                title="Server Health",
            ))
    asyncio.run(run())


# ── Orchestrate ───────────────────────────────────────────────────────

@cli.command()
@click.option("--conversation-id", type=int, required=True)
@click.option("--query", required=True)
@click.pass_context
def orchestrate(ctx: click.Context, conversation_id: int, query: str) -> None:
    """Send a query to the agent system."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        result = await client.orchestrate(conversation_id, query)
        if ctx.obj["JSON"]:
            click.echo(json.dumps({"response": result}))
        else:
            console.print(Panel(result, title=f"Orchestration Response (conv #{conversation_id})"))
    asyncio.run(run())


# ── Messages ──────────────────────────────────────────────────────────

@cli.group()
def messages() -> None:
    """Message operations."""


@messages.command("list")
@click.option("--conversation-id", type=int, required=True)
@click.pass_context
def list_messages(ctx: click.Context, conversation_id: int) -> None:
    """List messages in a conversation."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        result = await client.messages.list(conversation_id)
        if ctx.obj["JSON"]:
            click.echo(json.dumps([r.model_dump(mode="json") for r in result], default=str))
        else:
            for msg in result:
                role_color = "green" if msg.role == "user" else "blue"
                console.print(f"[bold {role_color}]{msg.role}[/bold {role_color}]: {msg.content[:200]}")
                console.print(f"  [dim]#{msg.id} at {msg.created_at}[/dim]")
                console.print()
    asyncio.run(run())


# ── Models ────────────────────────────────────────────────────────────

@cli.command()
@click.pass_context
def models(ctx: click.Context) -> None:
    """List available models."""
    async def run() -> None:
        client = _make_client(ctx.obj["TOKEN"], ctx.obj["BASE_URL"])
        result = await client.system.models()
        if ctx.obj["JSON"]:
            click.echo(json.dumps(result))
        else:
            for m in result:
                console.print(f"  • [cyan]{m}[/cyan]")
    asyncio.run(run())


# ── Main ──────────────────────────────────────────────────────────────

if __name__ == "__main__":
    cli()
