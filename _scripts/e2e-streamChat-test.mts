// E2E test for the frontend SSE parsing path (runtime.ts streamChat).
// Runs the REAL streamChat function against the REAL apeireth-api :8080
// (which stream_forwards to the mock upstream :9999). Verifies the UI-side
// parsing chain that App.svelte relies on.
import {streamChat} from '../frontend/companion-desktop/src/lib/runtime.ts';

// Node 24 has global fetch + AbortController. streamChat needs a Response-like
// body reader; native fetch provides ReadableStream. Good.

const BASE = 'http://127.0.0.1:8080';
const API_KEY = 'testmock'; // arbitrary non-empty; real key held by backend

async function testStreaming(): Promise<boolean> {
  console.log('--- TEST: streaming delta accumulation ---');
  let accumulated = '';
  const deltas: string[] = [];
  const full = await streamChat(
    {baseUrl: BASE, apiKey: API_KEY, model: 'mock'},
    [{role: 'user', content: 'APEIRETH_E2E_OK'}],
    (delta) => {
      deltas.push(delta);
      accumulated += delta;
    },
  );
  console.log(`  accumulated: "${accumulated}"`);
  console.log(`  full return: "${full}"`);
  console.log(`  delta count: ${deltas.length}`);
  const ok =
    full === 'APEIRETH_E2E_OK' &&
    accumulated === 'APEIRETH_E2E_OK' &&
    deltas.length > 1;
  console.log(`  PASS: ${ok}`);
  return ok;
}

async function testCancellation(): Promise<boolean> {
  console.log('--- TEST: cancellation via AbortSignal (mid-stream) ---');
  const controller = new AbortController();
  let gotAbort = false;
  let receivedBefore = 0;

  // Abort after the first delta arrives, while the stream is still flowing.
  const promise = streamChat(
    {baseUrl: BASE, apiKey: API_KEY, model: 'mock'},
    [{role: 'user', content: '这是一个需要足够长的回复以便在流式中途取消的测试消息，请给出很长的回答。'}],
    (delta) => {
      receivedBefore += 1;
      if (receivedBefore === 1) {
        // Fire abort on the very first delta — mid-stream.
        setTimeout(() => controller.abort(), 5);
      }
      void delta;
    },
    controller.signal,
  );

  try {
    await promise;
    console.log('  streamChat resolved (unexpected — should have aborted)');
    return false;
  } catch (caught) {
    gotAbort = caught instanceof DOMException && caught.name === 'AbortError';
    console.log(`  caught: ${caught instanceof Error ? caught.name : String(caught)}`);
  }
  console.log(`  gotAbort: ${gotAbort}`);
  console.log(`  deltas received before abort: ${receivedBefore}`);
  // After abort, the reader should be closed and no more deltas.
  await new Promise((r) => setTimeout(r, 400));
  console.log(`  deltas total after wait: ${receivedBefore}`);
  return gotAbort;
}

async function testError(): Promise<boolean> {
  console.log('--- TEST: unreachable backend → TypeError (network) ---');
  try {
    await streamChat(
      {baseUrl: 'http://127.0.0.1:59999', apiKey: 'x', model: 'm'},
      [{role: 'user', content: 'hi'}],
      () => {},
    );
    console.log('  NO ERROR (unexpected)');
    return false;
  } catch (caught) {
    console.log(`  caught: ${caught instanceof Error ? caught.name : String(caught)}`);
    const isTypeError = caught instanceof TypeError;
    console.log(`  is TypeError: ${isTypeError}`);
    return isTypeError;
  }
}

async function testErrorStatus(status: number, model: string, expectedCode: string): Promise<boolean> {
  console.log(`--- TEST: upstream ${status} → classified ${expectedCode} ---`);
  try {
    await streamChat(
      {baseUrl: BASE, apiKey: API_KEY, model},
      [{role: 'user', content: 'hi'}],
      () => {},
    );
    console.log(`  NO ERROR (unexpected for ${status})`);
    return false;
  } catch (caught) {
    const {toRuntimeError} = await import('../frontend/companion-desktop/src/lib/runtime.ts');
    const err = toRuntimeError(caught);
    console.log(`  classified: ${err.code} / status=${err.status} / ${err.message.slice(0, 80)}`);
    return err.code === expectedCode && err.status === status;
  }
}

async function main(): Promise<void> {
  const results: Array<[string, boolean]> = [];
  try {
    results.push(['streaming', await testStreaming()]);
  } catch (e) {
    console.log('  streaming threw: ' + e);
    results.push(['streaming', false]);
  }

  // Cancellation: separate run so the abort timeout applies to a fresh stream
  try {
    results.push(['cancellation', await testCancellation()]);
  } catch (e) {
    console.log('  cancellation threw: ' + e);
    results.push(['cancellation', false]);
  }

  try {
    results.push(['error-network', await testError()]);
  } catch (e) {
    console.log('  error threw: ' + e);
    results.push(['error-network', false]);
  }

  try {
    results.push(['error-401-auth', await testErrorStatus(401, 'fail-401', 'auth')]);
  } catch (e) {
    console.log('  error-401 threw: ' + e);
    results.push(['error-401-auth', false]);
  }

  try {
    results.push(['error-500-http', await testErrorStatus(500, 'fail-500', 'http')]);
  } catch (e) {
    console.log('  error-500 threw: ' + e);
    results.push(['error-500-http', false]);
  }

  console.log('\n===== SUMMARY =====');
  for (const [name, ok] of results) {
    console.log(`  ${name}: ${ok ? 'PASS' : 'FAIL'}`);
  }
  const all = results.every(([, ok]) => ok);
  console.log(`\nALL: ${all ? 'PASS' : 'FAIL'}`);
  process.exit(all ? 0 : 1);
}

main();
