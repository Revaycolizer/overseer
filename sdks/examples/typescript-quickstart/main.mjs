// From the repository root: npm --prefix sdks/typescript run build
// Then: node sdks/examples/typescript-quickstart/main.mjs
import { OverseerClient } from '../../typescript/dist/index.js';
const client = new OverseerClient({
  baseUrl: process.env.OVERSEER_BASE_URL ?? 'http://127.0.0.1:8080',
  token: process.env.OVERSEER_API_TOKEN,
});
console.log('Health:', await client.system.health());
console.log('Conversations:', await client.conversations.list(10));
// Explicit opt-in to create a conversation and run a memory command.
if (process.argv.includes('--write')) {
  const conversationId = await client.conversations.create('JavaScript SDK');
  for await (const event of client.chat.stream({ conversationId, message: '/memory list' })) {
    if (event.type === 'done') console.log(event.response);
  }
}
