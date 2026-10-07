import asyncio
import json
from contextlib import aclosing
import httpx
import pytest
from websockets.asyncio.server import serve
from overseer import OverseerClient

ID = 9223372036854775806

@pytest.mark.asyncio
async def test_exact_ids_and_current_response_shapes():
    requests = []
    def handler(request):
        requests.append(request)
        path = request.url.path
        if path == '/api/conversations' and request.method == 'POST':
            return httpx.Response(200, json=ID)
        if path == '/api/conversations':
            return httpx.Response(200, json=[dict(id=ID, title='SDK', created_at='2026-10-05T00:00:00')])
        if path == '/api/system/load':
            return httpx.Response(200, json=dict(label='System Load', usage=0))
        if path == '/api/registry/status':
            return httpx.Response(200, json=dict(status='live', refreshed_at=None, error=None))
        if path == '/api/orchestrate':
            return httpx.Response(200, text='plain answer')
        if path == '/api/conversations/search':
            return httpx.Response(200, json=[dict(conversation_id=ID, conversation_title='SDK', message_id=ID-1, content='answer', created_at='2026-10-05T00:00:00')])
        return httpx.Response(204)
    async with OverseerClient(token='test', transport=httpx.MockTransport(handler)) as client:
        assert await client.conversations.create('SDK') == ID
        assert (await client.conversations.list())[0].id == ID
        await client.conversations.update_title(ID, 'updated')
        assert json.loads(requests[-1].content)['id'] == ID
        assert (await client.system.load()).label == 'System Load'
        assert (await client.system.registry_status()).status == 'live'
        assert (await client.conversations.search('answer'))[0].content == 'answer'
        assert await client.orchestrate(ID, 'hello') == 'plain answer'
        assert all(r.headers['authorization'] == 'Bearer test' for r in requests)
        client.token = None
        await client.conversations.list()
        assert 'authorization' not in requests[-1].headers

@pytest.mark.asyncio
async def test_error_redirect_and_origin_guard():
    async with OverseerClient(transport=httpx.MockTransport(lambda r: httpx.Response(401, text='denied'))) as client:
        with pytest.raises(httpx.HTTPStatusError, match='denied'):
            await client.agents.list()
        with pytest.raises(ValueError):
            await client.request('GET', '//other.example/path')
    async with OverseerClient(transport=httpx.MockTransport(lambda r: httpx.Response(302, headers={'location': 'https://example.invalid'}))) as client:
        with pytest.raises(httpx.HTTPStatusError):
            await client.agents.list()

@pytest.mark.asyncio
@pytest.mark.parametrize('approval', [False, True])
async def test_stream_protocol_with_real_websocket(approval):
    received = []
    async def handler(socket):
        received.append(json.loads(await socket.recv()))
        received.append(json.loads(await socket.recv()))
        await socket.send(json.dumps(dict(type='session', conversation_id=str(ID))))
        await socket.send(json.dumps(dict(type='token', text='hello')))
        await socket.send(json.dumps(dict(type='approval', id='approval-1', tool='read_file', kind='read', summary='read', reason='review')))
        received.append(json.loads(await socket.recv()))
        await socket.send(json.dumps(dict(type='done', response='finished')))
    async with serve(handler, '127.0.0.1', 0) as server:
        port = server.sockets[0].getsockname()[1]
        async with OverseerClient(f'http://127.0.0.1:{port}', token='test') as client:
            events = [event async for event in client.chat.stream('hello', ID, on_approval=(lambda event: True) if approval else None)]
    assert events[-1]['response'] == 'finished'
    assert received == [{'token': 'test'}, {'conversation_id': str(ID), 'message': 'hello'}, {'approve' if approval else 'reject': 'approval-1'}]

@pytest.mark.asyncio
async def test_stream_timeout_and_early_exit_close_connection():
    closed = asyncio.Event()
    async def handler(socket):
        await socket.recv()
        await socket.send(json.dumps(dict(type='token', text='hello')))
        await socket.wait_closed()
        closed.set()
    async with serve(handler, '127.0.0.1', 0) as server:
        async with OverseerClient(f'http://127.0.0.1:{server.sockets[0].getsockname()[1]}') as client:
            async with aclosing(client.chat.stream('hello')) as stream:
                async for event in stream:
                    break
            await asyncio.wait_for(closed.wait(), 1)
            async with aclosing(client.chat.stream('hello', timeout=.03)) as stream:
                with pytest.raises(asyncio.TimeoutError):
                    async for event in stream:
                        pass
