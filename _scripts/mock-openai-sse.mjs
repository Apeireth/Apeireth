// Mock OpenAI-compatible SSE server for E2E testing.
// Simulates a streaming chat completion upstream so the full chain
// UI → AgentRuntime → apeireth-api (stream_forward) → upstream can be verified
// without a real model key. This is a TEST DOUBLE for the *upstream provider*,
// NOT a fake Apeireth backend capability.
//
// Responds to POST /v1/chat/completions with standard OpenAI SSE chunks.
import {createServer} from 'node:http';

const PORT = Number(process.env.MOCK_PORT || 9999);

function sse(data) {
  return `data: ${JSON.stringify(data)}\n\n`;
}

const server = createServer((req, res) => {
  const chunks = [];
  req.on('data', (c) => chunks.push(c));
  req.on('end', () => {
    let body = {};
    try {
      body = JSON.parse(Buffer.concat(chunks).toString() || '{}');
    } catch {
      body = {};
    }
    const model = body.model || 'mock-model';

    if (req.url === '/v1/chat/completions' && req.method === 'POST') {
      // Fault injection: model name drives error simulation for E2E error-path tests
      if (model === 'fail-401') {
        res.writeHead(401, {'content-type': 'application/json'});
        res.end(JSON.stringify({error: {message: 'invalid api key (mock)', type: 'invalid_request_error', code: 'invalid_api_key'}}));
        return;
      }
      if (model === 'fail-500') {
        res.writeHead(500, {'content-type': 'application/json'});
        res.end(JSON.stringify({error: {message: 'upstream exploded (mock)', type: 'server_error'}}));
        return;
      }
      if (model === 'fail-timeout') {
        // Hang forever so the client-side abort/timeout path can be exercised
        return;
      }
      const stream = body.stream !== false;

      // Determine response: echo test marker or a canned long reply
      const lastUser = (body.messages || []).filter((m) => m.role === 'user').at(-1);
      const prompt = lastUser?.content || '';
      let final = 'APEIRETH_E2E_OK';
      if (!prompt.includes('APEIRETH_E2E_OK')) {
        final = `这是来自 mock 上游的回复。你问我: ${String(prompt).slice(0, 40)}。`.repeat(3);
      }

      if (!stream) {
        res.writeHead(200, {'content-type': 'application/json'});
        res.end(JSON.stringify({
          id: 'chatcmpl-mock',
          object: 'chat.completion',
          created: Math.floor(Date.now() / 1000),
          model,
          choices: [{index: 0, message: {role: 'assistant', content: final}, finish_reason: 'stop'}],
          usage: {prompt_tokens: 1, completion_tokens: 2, total_tokens: 3},
        }));
        return;
      }

      // Streaming SSE with staggered deltas
      res.writeHead(200, {
        'content-type': 'text/event-stream',
        'cache-control': 'no-cache',
        connection: 'keep-alive',
      });
      res.write(sse({id: 'chatcmpl-mock', object: 'chat.completion.chunk', created: Math.floor(Date.now() / 1000), model, choices: [{index: 0, delta: {role: 'assistant'}, finish_reason: null}]}));
      let i = 0;
      const tick = () => {
        if (i >= final.length) {
          res.write(sse({id: 'chatcmpl-mock', object: 'chat.completion.chunk', created: Math.floor(Date.now() / 1000), model, choices: [{index: 0, delta: {}, finish_reason: 'stop'}]}));
          res.write('data: [DONE]\n\n');
          res.end();
          return;
        }
        const n = Math.min(4, final.length - i);
        const piece = final.slice(i, i + n);
        i += n;
        res.write(sse({id: 'chatcmpl-mock', object: 'chat.completion.chunk', created: Math.floor(Date.now() / 1000), model, choices: [{index: 0, delta: {content: piece}, finish_reason: null}]}));
        setTimeout(tick, 25); // ~40 chunks/s → slow enough to observe streaming
      };
      setTimeout(tick, 50);
      return;
    }

    if (req.url === '/v1/models') {
      res.writeHead(200, {'content-type': 'application/json'});
      res.end(JSON.stringify({object: 'list', data: [{id: model, object: 'model', owned_by: 'mock'}]}));
      return;
    }

    res.writeHead(404, {'content-type': 'application/json'});
    res.end(JSON.stringify({error: {message: 'not found'}}));
  });
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`mock openai sse upstream on http://127.0.0.1:${PORT}/v1`);
});
