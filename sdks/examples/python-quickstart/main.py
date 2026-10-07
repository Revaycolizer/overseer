#!/usr/bin/env python3
"""Overseer Python SDK Quickstart.

Run:
    pip install -e sdks/python
    python main.py

Or from repo root:
    pip install -e sdks/python
    python sdks/examples/python-quickstart/main.py
"""

import asyncio
import os

from overseer import OverseerClient


async def main():
    # 1. Connect to the server
    token = os.environ.get("OVERSEER_API_TOKEN")
    async with OverseerClient("http://localhost:8080", token=token) as client:
        # 2. Check health
        health = await client.system.health()
        print(f"Server: {health.status} (v{health.version})")

        # 3. List conversations
        conversations = await client.conversations.list(limit=10)
        print("\nConversations:")
        for conv in conversations:
            print(f"  #{conv.id}: {conv.title}")

        # 4. List agents
        agents = await client.agents.list()
        print("\nAgents:")
        for agent in agents:
            print(f"  {agent.name} ({agent.domain})")

        # 5. List tools
        tools = await client.tools.list()
        print("\nTools:")
        for tool in tools:
            print(f"  {tool.name}: {tool.description}")

        # 6. Create conversation and send a message
        if not conversations:
            conv_id = await client.conversations.create("SDK Quickstart")
            print(f"\nCreated conversation #{conv_id}")

            msg_id = await client.messages.send(
                conversation_id=conv_id,
                role="user",
                content="Hello from the Python SDK!",
            )
            print(f"Sent message #{msg_id}")

            messages = await client.messages.list(conversation_id=conv_id)
            print(f"\nMessages in conversation #{conv_id}:")
            for msg in messages:
                print(f"  [{msg.role}] {msg.content}")


if __name__ == "__main__":
    asyncio.run(main())
