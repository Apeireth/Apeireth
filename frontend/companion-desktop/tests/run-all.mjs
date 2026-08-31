#!/usr/bin/env node
// Canonical frontend test runner for Apeireth 2.0 desktop.
//
// `pnpm test` previously did not exist, so the suites below were never run in
// any validation pass. This runner wires the ones that assert against the
// canonical 2.0 contract (no backend process required) into a single command.
//
// Excluded deliberately, with reasons:
//   frontend-smoke.cjs      needs Playwright + a live desktop on :1420 and
//                           asserts historical /v1/panel/* routes — it validates
//                           the retired companion backend, not canonical 2.0.
//   e2e-streamChat-test.mts needs a live gateway on :8080; belongs to the
//                           packaged/live E2E gate, not the unit gate.
//   mock-openai-sse.mjs     a server fixture, not a test.

import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {dirname, join} from 'node:path';

const testsDir = dirname(fileURLToPath(import.meta.url));

/** `strip` marks suites that import .ts sources and need type stripping. */
const SUITES = [
  // Imports the REAL src/lib/runtime.ts, so a regression in the production
  // release contract or error surface fails here. The two suites below it
  // mirror their logic locally and cannot catch that.
  {file: 'release-contract.mjs', name: 'canonical release contract (real module)', strip: true},
  {file: 'capability-manifest.mjs', name: 'capability manifest projection (mirrored)'},
  {file: 'desktop-capability-gating.mjs', name: 'capability gating (mirrored)'},
  {file: 'reality-check.mjs', name: 'storage + secret-persistence safety'},
  {file: 'security-attack.mjs', name: 'redaction / spoofing defence'},
  {file: 'presence-split.mjs', name: 'presence frame parsing', strip: true},
];

function run({file, strip}) {
  const args = strip ? ['--experimental-strip-types', join(testsDir, file)] : [join(testsDir, file)];
  return new Promise((resolve) => {
    const child = spawn(process.execPath, args, {stdio: ['ignore', 'pipe', 'pipe']});
    let output = '';
    child.stdout.on('data', (chunk) => (output += chunk));
    child.stderr.on('data', (chunk) => (output += chunk));
    child.on('close', (code) => resolve({code, output}));
    child.on('error', (error) => resolve({code: 1, output: String(error)}));
  });
}

const failures = [];
for (const suite of SUITES) {
  const {code, output} = await run(suite);
  const ok = code === 0;
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${suite.name}  (${suite.file})`);
  if (!ok) failures.push({suite, output});
}

console.log('');
if (failures.length > 0) {
  for (const {suite, output} of failures) {
    console.error(`--- ${suite.file} ---`);
    console.error(output.trimEnd());
    console.error('');
  }
  console.error(`${failures.length} of ${SUITES.length} suites failed.`);
  process.exit(1);
}

console.log(`${SUITES.length} of ${SUITES.length} suites passed.`);
