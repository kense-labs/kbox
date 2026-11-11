// A real stdio MCP server with a per-run challenge and an independent call receipt.
import fs from 'node:fs';
import readline from 'node:readline';

const challenge = process.env.ACCEPTANCE_CHALLENGE;
if (!challenge) throw new Error('missing challenge');
for await (const line of readline.createInterface({input: process.stdin})) {
  const request = JSON.parse(line);
  if (request.id === undefined) continue;
  let result;
  switch (request.method) {
    case 'initialize':
      result = {protocolVersion: request.params.protocolVersion, capabilities: {tools: {}},
        serverInfo: {name: 'kbox-acceptance', version: '1'}};
      break;
    case 'ping':
      result = {};
      break;
    case 'tools/list':
      result = {tools: [{name: 'acceptance_challenge', description: 'Return this live server\'s challenge.',
        inputSchema: {type: 'object', properties: {}, additionalProperties: false}}]};
      break;
    case 'tools/call':
      if (request.params.name !== 'acceptance_challenge') throw new Error('unknown tool');
      fs.writeFileSync('/external/mcp-called.json', JSON.stringify({challenge}));
      result = {content: [{type: 'text', text: challenge}], isError: false};
      break;
    default:
      process.stdout.write(JSON.stringify({jsonrpc: '2.0', id: request.id,
        error: {code: -32601, message: 'unknown MCP method'}}) + '\n');
      continue;
  }
  process.stdout.write(JSON.stringify({jsonrpc: '2.0', id: request.id, result}) + '\n');
}
