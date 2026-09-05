// Canonical release-contract tests — imports the REAL src/lib/runtime.ts.
//
// The older capability suites (capability-manifest.mjs, desktop-capability-gating.mjs)
// declare their own local copies of legacyCapabilityManifest/capabilitySupported and
// assert against those. That means they pass whatever the production module does,
// so a regression in the real manifest cannot fail them. This suite closes that
// hole by importing the module under test.
//
// Run: node --experimental-strip-types tests/release-contract.mjs
import assert from 'node:assert/strict';

const runtime = await import('../src/lib/runtime.ts');
const {
  releaseContractManifest,
  legacyCapabilityManifest,
  capabilitySupported,
  capabilityAvailable,
  loadConfig,
  saveConfig,
  friendlyErrorMessage,
  HttpError,
  toRuntimeError,
} = runtime;

let checks = 0;
const check = (name, fn) => {
  fn();
  checks += 1;
  console.log(`  ok  ${name}`);
};

console.log('--- Canonical release contract (real module) ---');

const storage = new Map();
globalThis.localStorage = {
  getItem: (key) => storage.get(key) ?? null,
  setItem: (key, value) => storage.set(key, String(value)),
  removeItem: (key) => storage.delete(key),
};

// The routes canonical_entry.rs actually registers.
const CANONICAL = ['health', 'models.list', 'chat.completions', 'permissions.approval.resolve'];

// Optional runtime projections are deliberately not assumed when discovery is
// unavailable. The live manifest can enable them after a successful fetch.
const UNSUPPORTED = [
  'sessions.read',
  'memory.read',
  'memory.graph.read',
  'memory.write',
  'memory.forget',
  'memory.protect',
  'tools.list',
  'audit.read',
  'trace.read',
  'activity.audit',
  'permissions.grants.read',
  'permissions.revoke',
];

check('canonical capabilities are supported', () => {
  const manifest = releaseContractManifest();
  for (const id of CANONICAL) {
    assert.equal(capabilitySupported(manifest, id), true, `${id} should be supported`);
  }
});

check('approvals.resolve remains a compatibility alias of permissions.approval.resolve', () => {
  const manifest = releaseContractManifest();
  assert.equal(capabilitySupported(manifest, 'approvals.resolve'), true);
});

check('no introspection capability is claimed', () => {
  const manifest = releaseContractManifest();
  for (const id of UNSUPPORTED) {
    assert.equal(
      capabilitySupported(manifest, id),
      false,
      `${id} must NOT be claimed — canonical 2.0 exposes no such API`,
    );
  }
});

check('unknown capability ids are unsupported', () => {
  const manifest = releaseContractManifest();
  for (const id of ['', 'nonsense', 'memory', 'chat', 'future.capability']) {
    assert.equal(capabilitySupported(manifest, id), false, `${id} must default to unsupported`);
  }
});

check('null manifest gates everything off', () => {
  for (const id of [...CANONICAL, ...UNSUPPORTED]) {
    assert.equal(capabilitySupported(null, id), false);
    assert.equal(capabilityAvailable(null, id), false);
  }
});

check('availability falls back to supported', () => {
  const manifest = releaseContractManifest();
  for (const id of CANONICAL) {
    assert.equal(capabilityAvailable(manifest, id), true, `${id} should be available`);
  }
  for (const id of UNSUPPORTED) {
    assert.equal(capabilityAvailable(manifest, id), false, `${id} should be unavailable`);
  }
});

check('availability never overrides unsupported status', () => {
  const manifest = {
    schema_version: 1,
    runtime: {service: 'test', version: 'test'},
    capabilities: [{name: 'test', capabilities: [{id: 'test.cap', supported: false, available: true}]}],
  };
  assert.equal(capabilitySupported(manifest, 'test.cap'), false);
  assert.equal(capabilityAvailable(manifest, 'test.cap'), false);
});

check('manifest is marked as not runtime-sourced', () => {
  const manifest = releaseContractManifest();
  assert.equal(manifest.legacy, true, 'must be flagged as not from the runtime itself');
  assert.equal(typeof manifest.schema_version, 'number');
  assert.ok(Array.isArray(manifest.capabilities));
  assert.ok(manifest.runtime && typeof manifest.runtime.service === 'string');
});

check('deprecated alias still resolves to the same contract', () => {
  assert.equal(
    JSON.stringify(legacyCapabilityManifest()),
    JSON.stringify(releaseContractManifest()),
    'the retained alias must not reintroduce the permissive profile',
  );
});

check('config persistence strips nested credentials', () => {
  saveConfig({
    baseUrl: 'http://127.0.0.1:8080',
    apiKey: 'top-level-secret',
    model: 'x',
    provider: {
      protocol: 'openai',
      preset: 'custom',
      baseUrl: 'https://provider.invalid',
      apiKey: 'nested-secret',
      model: 'x',
      metadata: {accessToken: 'deep-secret', safe: 'kept'},
    },
  });
  const saved = storage.get('apeireth-config');
  assert.equal(saved.includes('secret'), false);
  assert.equal(saved.includes('accessToken'), false);

  storage.set('apeireth-config', JSON.stringify({
    baseUrl: 'http://127.0.0.1:8080',
    model: 'x',
    provider: {protocol: 'openai', apiKey: 'legacy-secret', token: 'legacy-token'},
  }));
  const loaded = loadConfig();
  assert.equal(loaded.apiKey, '');
  assert.equal(loaded.provider.apiKey, '');
  const migrated = storage.get('apeireth-config');
  assert.equal(migrated.includes('legacy-secret'), false);
  assert.equal(migrated.includes('legacy-token'), false);
});

console.log('--- Error surface ---');

check('legacy 404 is explained, not reported as a connection failure', () => {
  const message = friendlyErrorMessage(new HttpError(404, 'HTTP 404'), '/v1/panel/sessions');
  assert.ok(message.includes('不支持'), `expected an unsupported explanation, got: ${message}`);
  assert.ok(!/\[object Object\]/.test(message));
});

check('real backend errors keep their message', () => {
  const backend = 'provider.minimax: authentication failed: missing API key';
  const message = friendlyErrorMessage(new HttpError(502, backend), '/v1/chat/completions');
  assert.ok(message.includes(backend), `backend detail must survive, got: ${message}`);
});

check('thrown plain objects never render as [object Object]', () => {
  for (const thrown of [
    {message: 'plain object with message'},
    {code: 'x'},
    'a bare string',
    42,
    null,
    undefined,
  ]) {
    const viaFriendly = friendlyErrorMessage(thrown, '/v1/chat/completions');
    assert.ok(
      !/\[object Object\]/.test(viaFriendly),
      `friendlyErrorMessage leaked [object Object] for ${JSON.stringify(thrown)}`,
    );
    const viaRuntime = toRuntimeError(thrown).message;
    assert.ok(
      !/\[object Object\]/.test(viaRuntime),
      `toRuntimeError leaked [object Object] for ${JSON.stringify(thrown)}`,
    );
  }
});

check('HTTP status codes map to distinguishable errors', () => {
  for (const status of [400, 502, 503]) {
    const error = toRuntimeError(new HttpError(status, `HTTP ${status}: backend detail`));
    assert.equal(error.status, status, `status ${status} must be preserved`);
    assert.ok(error.message.includes('backend detail'), 'backend detail must survive');
  }
});

console.log('');
console.log(`${checks} checks passed.`);
