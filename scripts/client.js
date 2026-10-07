#!/usr/bin/env node
/**
 * Overseer REST API Client — single-file JavaScript implementation using native fetch.
 *
 * Usage:
 *   node scripts/client.js --help
 *
 * Examples:
 *   export OVESEER_TOKEN=$(node scripts/client.js login --username admin --password secret)
 *   node scripts/client.js conversations list
 *   node scripts/client.js agents list
 */

const BASE_URL = process.env.OVERSEER_BASE_URL || process.env.OVESEER_BASE_URL || 'http://localhost:8080';
let TOKEN = process.env.OVERSEER_API_TOKEN || process.env.OVESEER_TOKEN || '';

// ── HTTP helpers ──────────────────────────────────────────────────────

async function request(method, path, body) {
  const url = new URL(path, BASE_URL);
  const headers = { 'Accept': 'application/json', 'User-Agent': 'overseer-client-js/0.1' };
  if (TOKEN) headers['Authorization'] = `Bearer ${TOKEN}`;
  const opts = { method, headers };
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json';
    opts.body = JSON.stringify(body);
  }
  const r = await fetch(url, opts);
  if (!r.ok) {
    const text = await r.text();
    throw new Error(`API error ${r.status}: ${text.slice(0, 200)}`);
  }
  const ct = r.headers.get('content-type') || '';
  if (ct.includes('application/json')) return r.json();
  return r.text();
}

const get = (path) => request('GET', path);
const post = (path, body) => request('POST', path, body);
const del = (path) => request('DELETE', path);

// ── Commands ──────────────────────────────────────────────────────────

const commands = {};

commands.login = async (args) => {
  const username = args._[0] || args.username;
  const password = args._[1] || args.password;
  if (!username || !password) throw new Error('Usage: login <username> <password>');
  const r = await post('/v1/auth/login', { username, password });
  TOKEN = r.token;
  console.log(r.token);
};

commands.conversations = async (args) => {
  const action = args._[0] || 'list';
  switch (action) {
    case 'list': {
      const r = await get('/api/conversations?limit=50');
      console.log(JSON.stringify(r, null, 2));
      break;
    }
    case 'create': {
      const title = args._[1] || args.title;
      if (!title) throw new Error('Usage: conversations create <title>');
      const r = await post('/api/conversations', { title });
      console.log(JSON.stringify({ id: r }, null, 2));
      break;
    }
    case 'delete': {
      const id = parseInt(args._[1] || args.id);
      if (!id) throw new Error('Usage: conversations delete <id>');
      await post('/api/conversations/delete', { id });
      console.log(`Deleted conversation #${id}`);
      break;
    }
    default:
      throw new Error(`Unknown action: ${action}`);
  }
};

commands.agents = async () => {
  const r = await get('/api/agents');
  console.log(JSON.stringify(r, null, 2));
};

commands.tools = async () => {
  const r = await get('/api/tools');
  console.log(JSON.stringify(r, null, 2));
};

commands.workflows = async () => {
  const r = await get('/api/workflows');
  console.log(JSON.stringify(r, null, 2));
};

commands.messages = async (args) => {
  const action = args._[0] || 'list';
  const conversationId = parseInt(args._[1] || args['conversation-id']);
  if (!conversationId) throw new Error('Usage: messages <list|send> <conversation-id> [content]');
  switch (action) {
    case 'list': {
      const r = await get(`/api/messages?conversation_id=${conversationId}`);
      console.log(JSON.stringify(r, null, 2));
      break;
    }
    case 'send': {
      const content = args._[2] || args.content;
      if (!content) throw new Error('Usage: messages send <conversation-id> <content>');
      const r = await post('/api/messages', { conversation_id: conversationId, role: 'user', content, hide_evidence: false });
      console.log(JSON.stringify({ message_id: r }, null, 2));
      break;
    }
    default:
      throw new Error(`Unknown action: ${action}`);
  }
};

commands.health = async () => {
  const r = await get('/health');
  console.log(JSON.stringify(r, null, 2));
};

commands.models = async () => {
  const r = await get('/api/models/available');
  console.log(JSON.stringify(r, null, 2));
};

commands.orchestrate = async (args) => {
  const conversationId = parseInt(args._[0] || args['conversation-id']);
  const query = args._[1] || args.query;
  if (!conversationId || !query) throw new Error('Usage: orchestrate <conversation-id> <query>');
  const r = await post('/api/orchestrate', { conversation_id: conversationId, query });
  console.log(typeof r === 'string' ? r : JSON.stringify(r, null, 2));
};

// ── CLI ───────────────────────────────────────────────────────────────

function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const [k, v] = a.slice(2).split('=');
      if (v !== undefined) args[k] = v;
      else if (i + 1 < argv.length && !argv[i + 1].startsWith('--')) {
        args[k] = argv[++i];
      } else args[k] = true;
    } else {
      args._.push(a);
    }
  }
  return args;
}

async function main() {
  const args = parseArgs(process.argv);
  const cmd = args._.shift();
  if (!cmd || cmd === 'help' || cmd === '--help') {
    console.log(`
Overseer API Client (JavaScript)

Usage:
  node scripts/client.js <command> [args...]

Commands:
  login <username> <password>       Login and print token
  conversations list                 List conversations
  conversations create <title>       Create conversation
  conversations delete <id>          Delete conversation
  agents                             List agents
  tools                              List tools
  workflows                          List workflows
  messages list <conv-id>            List messages
  messages send <conv-id> <content>  Send message
  health                             Server health
  models                             Available models
  orchestrate <conv-id> <query>      Send query to agent

Environment:
  OVESEER_BASE_URL  Server URL (default: http://localhost:8080)
  OVESEER_TOKEN     Bearer token
`);
    return;
  }
  if (commands[cmd]) {
    await commands[cmd](args);
  } else {
    console.error(`Unknown command: ${cmd}`);
    process.exit(1);
  }
}

main().catch(err => { console.error(err.message); process.exit(1); });
