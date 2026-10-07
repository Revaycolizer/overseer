"""Only launched by verify-live.py against a temporary daemon/database."""
import asyncio, os
import httpx
from overseer import OverseerClient
async def main():
    url=os.environ['OVERSEER_BASE_URL']
    async with OverseerClient(url,token=os.environ['OVERSEER_API_TOKEN']) as client:
        assert (await client.system.health()).db_connected
        id=await client.conversations.create('Python SDK verification')
        assert id>9007199254740991
        message=await client.messages.send(id,'user','SDK contract check')
        assert any(m.id==message for m in await client.messages.list(id))
        assert any(c.id==id for c in await client.conversations.list())
        await client.system.runtime_health();await client.system.load();await client.system.registry_status()
        await client.agents.list();await client.tools.list();await client.supervision.get_state()
        workflow=await client.workflows.create('SDK verification')
        await client.workflows.list();step=await client.workflows.add_step(workflow)
        assert any(s.id==step for s in await client.workflows.get_steps(workflow))
        await client.workflows.delete(workflow)
        events=[event async for event in client.chat.stream('/memory list',id)]
        assert events[-1]['type']=='done'
        await client.conversations.delete(id)
    async with OverseerClient(url) as denied:
        try:await denied.conversations.list()
        except httpx.HTTPStatusError as error:assert error.response.status_code==401
        else:raise AssertionError('Missing token should fail')
    print('Python SDK: live REST, exact IDs, auth and WebSocket memory passed')
asyncio.run(main())
