// Tier 0: desktop observes the canonical tool/approval path and never hits dead v1 URLs.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';

const runtime = await import('../src/lib/runtime.ts');
const {
  fetchCapabilities,
  grantToolPermission,
  fetchApprovalRequests,
  fetchGrants,
  revokeGrant,
  subscribeCompanionEvents,
  streamChat,
  ApprovalRequiredError,
  classifyHttpError,
} = runtime;

const srcDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'src');
const calls = [];
const originalFetch = globalThis.fetch;
globalThis.fetch = async (input, init) => {
  const url = String(input);
  calls.push({url, method: init?.method || 'GET', body: init?.body});
  throw new Error(`unexpected fetch: ${url}`);
};

let checks = 0;
const check = (name, fn) => {
  fn();
  checks += 1;
  console.log(`  ok  ${name}`);
};

console.log('--- Tier 0 parity (real module) ---');

{
  calls.length = 0;
  const manifest = await fetchCapabilities({baseUrl: 'http://127.0.0.1:8080', model: 'x'});
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, 'http://127.0.0.1:8080/v1/apeireth/capabilities');
  assert.equal(manifest.runtime.service, 'apeireth-gateway-2.0');
  console.log('  ok  fetchCapabilities probes the manifest and falls back conservatively');
  checks += 1;
}

{
  calls.length = 0;
  globalThis.fetch = async (input, init) => {
    const url = String(input);
    calls.push({url, method: init?.method || 'GET', body: init?.body});
    if (url.endsWith('/v1/panel/grants')) {
      return {ok: true, status: 200, json: async () => ({grants: []})};
    }
    if (url.endsWith('/v1/panel/grants/revoke')) {
      return {ok: true, status: 200, json: async () => ({ok: true})};
    }
    throw new Error(`unexpected fetch: ${url}`);
  };
  const result = await grantToolPermission({baseUrl: 'http://127.0.0.1:8080', model: 'x'}, 'shell', 1, 'token');
  assert.equal(result.ok, false);
  assert.deepEqual(await fetchApprovalRequests({baseUrl: 'http://127.0.0.1:8080', model: 'x'}), []);
  assert.deepEqual(await fetchGrants({baseUrl: 'http://127.0.0.1:8080', model: 'x'}), []);
  assert.equal((await revokeGrant({baseUrl: 'http://127.0.0.1:8080', model: 'x'}, 'g1', 'token')).ok, true);
  assert.deepEqual(calls.map((call) => call.url), [
    'http://127.0.0.1:8080/v1/panel/grants',
    'http://127.0.0.1:8080/v1/panel/grants/revoke',
  ]);
  console.log('  ok  legacy grant mutation is inert; canonical grant projections use live routes');
  checks += 1;
}

check('subscribeCompanionEvents does not open /v1/apeireth/events', () => {
  calls.length = 0;
  const stop = subscribeCompanionEvents({baseUrl: 'http://127.0.0.1:8080', model: 'x'}, () => {});
  stop();
  assert.equal(calls.length, 0);
});

check('streamChat maps canonical events and never fakes success', async () => {
  calls.length = 0;
  globalThis.fetch = async () => ({
    ok: true,
    status: 200,
    body: {
      getReader() {
        const chunk = new TextEncoder().encode(
          'data: ' +
            JSON.stringify({
              choices: [{delta: {content: 'done'}, finish_reason: 'stop'}],
              apeireth: {
                events: [
                  {event: 'tool_started', tool_name: 'tool.repo', tool_call_id: 'c1'},
                  {event: 'tool_completed', tool_name: 'tool.repo', tool_call_id: 'c1', succeeded: true},
                ],
              },
            }) +
            '\n\ndata: [DONE]\n\n',
        );
        let sent = false;
        return {
          read: async () => {
            if (sent) return {done: true, value: undefined};
            sent = true;
            return {done: false, value: chunk};
          },
          releaseLock() {},
        };
      },
    },
  });
  const seen = [];
  const text = await streamChat(
    {baseUrl: 'http://127.0.0.1:8080', model: 'x'},
    [{role: 'user', content: 'hi'}],
    {
      onDelta: (t) => seen.push(['delta', t]),
      onToolCall: (tc) => seen.push(['call', tc.name, tc.status]),
      onToolResult: (id, ok, summary) => seen.push(['result', id, ok, summary]),
    },
  );
  assert.equal(text.includes('done'), true);
  assert.equal(seen.some((row) => row[0] === 'call'), true);
  assert.equal(seen.some((row) => row[0] === 'result' && row[2] === true), true);
  assert.equal(seen.some((row) => String(row[3]).includes('执行成功')), false);
});

check('streamChat 202 surfaces ApprovalRequiredError', async () => {
  globalThis.fetch = async () => ({
    ok: false,
    status: 202,
    json: async () => ({
      session: 's',
      approval_id: 'a',
      request: 'r',
      trace_id: 't',
      capability_id: 'tool.shell',
      tool_name: 'shell',
      governance_hook: 'permission',
      governance_reason: 'requires approval',
      created_at: 'now',
      expires_at: 'later',
    }),
  });
  await assert.rejects(
    () =>
      streamChat({baseUrl: 'http://127.0.0.1:8080', model: 'x'}, [{role: 'user', content: 'rm'}], {}),
    (error) => error instanceof ApprovalRequiredError && error.pending.tool_name === 'shell',
  );
});

check('error classes stay distinct', () => {
  assert.equal(classifyHttpError(403), 'denied');
  assert.equal(classifyHttpError(502), 'provider');
  assert.equal(classifyHttpError(503), 'backend');
  assert.equal(classifyHttpError(409), 'approval_required');
});

const appSrc = readFileSync(join(srcDir, 'App.svelte'), 'utf8');
check('App.svelte does not call v1 grant inbox', () => {
  assert.equal(appSrc.includes('fetchApprovalRequests'), false);
  assert.equal(appSrc.includes('/v1/apeireth/grant'), false);
  assert.equal(appSrc.includes('resolveCanonicalApproval'), true);
});

const activitySrc = readFileSync(join(srcDir, 'lib/views/ActivityView.svelte'), 'utf8');
check('ActivityView does not open SSE without activity.sse', () => {
  assert.equal(activitySrc.includes("capabilitySupported(capabilities, 'activity.sse')"), true);
});

const runtimeSrc = readFileSync(join(srcDir, 'lib/runtime.ts'), 'utf8');
check('production runtime no longer requests grant or fake success', () => {
  assert.equal(runtimeSrc.includes("callbacks.onToolResult?.(tc.id, true, '执行成功')"), false);
  assert.equal(/fetch\([^)]*\/v1\/apeireth\/grant/.test(runtimeSrc), false);
  assert.equal(/fetch\([^)]*\/v1\/apeireth\/capabilities/.test(runtimeSrc), true);
});

globalThis.fetch = originalFetch;
console.log(`--- ${checks} checks ---`);
