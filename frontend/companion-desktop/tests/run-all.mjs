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
  {file: 'presence-state.mjs', name: 'presence_state frame parsing', strip: true},
  {file: 'chat-shell.mjs', name: 'T0 chat shell logic (session ledger / approval events)', strip: true},
  {file: 'governance.mjs', name: 'governance ledger (approvals / guard / trace-audit)', strip: true},
  {file: 'commands.mjs', name: 'command palette registry (filter / alias / recent-first)', strip: true},
  {file: 'statusbar.mjs', name: 'status bar indicators (sse / turn / guard / memory)', strip: true},
  {file: 'sse-badge.mjs', name: 'sse badge three-state display (connected / connecting / offline, real module)', strip: true},
  {file: 'session-delete.mjs', name: 'session delete chain (optimistic remove / remote delete / rollback, real modules)', strip: true},
  {file: 'session-cleanup.mjs', name: 'session cleanup dual-scope purge (recent/all true-delete / partial-failure / copy red lines, real modules)', strip: true},
  {file: 'wheel-scroll.mjs', name: 'wheel router (dead-zone scroll routing / boundary chain / responsive settings, real module)', strip: true},
  {file: 'scrollbar-theme.mjs', name: 'scrollbar theme tokens (white-block fix / per-theme coverage / zero hardcode, source mirror)'},
  {file: 'scroll-policy.mjs', name: 'chat scroll policy (open-to-bottom / no yank back, real module)', strip: true},
  {file: 'shell-nav.mjs', name: 'sidebar consolidation targets (deep-link redirects / palette sync, real module)', strip: true},
  {file: 'shell-ia.mjs', name: 'shell wiring source mirrors (delete / new-session / rail four / workbench / settings security / sse reconnect)'},
  {file: 'memory-ledger.mjs', name: 'memory ledger (filters / revision lock / graph links)', strip: true},
  {file: 'theme-system.mjs', name: 'theme system (default/catalog/static-bg, real module)', strip: true},
  {file: 'theme-hud-switch.mjs', name: 'sci-fi HUD theme tier (instant switch / persistence / zero-regression, real module)', strip: true},
  {file: 'theme-hud-tokens.mjs', name: 'sci-fi HUD theme layer (token coverage / no hardcoded color / reduced-motion, source mirror)'},
  {file: 'personalization-bg.mjs', name: 'custom background upload validation (real module)', strip: true},
  {file: 'config-persistence.mjs', name: 'config persistence round-trip (accent/customBg/secret purge, real module)', strip: true},
  {file: 'first-run-onboarding.mjs', name: 'first-run onboarding wizard (tier split / casual presets / self-description, real module)', strip: true},
  {file: 'user-profile.mjs', name: 'user profile local persistence (normalize / round-trip / subscribe / honest write failure, real module)', strip: true},
  {file: 'recommended-preset.mjs', name: 'recommended preset ↔ memory-core knob mapping (real module)', strip: true},
  {file: 'capability-toggle-apply.mjs', name: 'capability toggle apply-on-click mapping (real module)', strip: true},
  {file: 'settings-live-apply.mjs', name: 'settings three-tier live apply (real module)', strip: true},
  {file: 'settings-save-buttons-removed.mjs', name: 'save-button removal source mirror'},
  {file: 'quick-window-close.mjs', name: 'quick window close/destroy lifecycle (mirrored)'},
  {file: 'quick-capture.mjs', name: 'quick capture (one-click memory stream / zero token / review list, real modules)', strip: true},
  {file: 'pet-frame.mjs', name: 'pet avatar contract (posture/light mirror, real module)', strip: true},
  {file: 'pet-proactive.mjs', name: 'pet proactive engine (anti-nagging budget / glance, real module)', strip: true},
  {file: 'pet-avatar-registry.mjs', name: 'pet avatar registry (dual-form plug-in slot, real module)', strip: true},
  {file: 'pet-window-lifecycle.mjs', name: 'pet window lifecycle + mutual exclusion (source mirror)'},
  {file: 'pet-character-mapping.mjs', name: 'pet character mapping (viseme/glance/blink discipline, real module)', strip: true},
  {file: 'pet-user-avatar.mjs', name: 'pet user avatar port (format gate / disclaimer discipline, real module)', strip: true},
  {file: 'pet-idle-behavior.mjs', name: 'pet idle micro-motion (amplitude cap / stretch window / doze ramp, real module)', strip: true},
  {file: 'self-tuning-mapping.mjs', name: 'self-tuning knob mapping (real module)', strip: true},
  {file: 'file-write-toggle.mjs', name: 'controlled file-write toggle mapping (real module)', strip: true},
  {file: 'provider-transport.mjs', name: 'provider transport selection (invoke vs fetch, real module)', strip: true},
  {file: 'tier0-parity.mjs', name: 'tier 0 tool/approval/dead-url parity', strip: true},
  {file: 'scroll-instant.mjs', name: 'chat scroll instant landing (no intermediate scroll frame, real module)', strip: true},
  {file: 'turn-telemetry.mjs', name: 'chat turn telemetry (usage wiring / honest dash placeholder, real module)', strip: true},
  {file: 'budget-knobs.mjs', name: 'budget knob face (clamp/fallback / effective badge / env wiring / quota registry, real modules)', strip: true},
  {file: 'settings-nav-groups.mjs', name: 'settings nav five groups (mapping / deep-link contract, real module)', strip: true},
  {file: 'budget-meter.mjs', name: 'budget meter (session accumulator / consumption card / remaining bars, real modules)', strip: true},
  {file: 'metrics-panel.mjs', name: 'metrics panel (contract view / dash semantics / auto refresh / fetch face, real modules)', strip: true},
  {file: 'daemon-ops.mjs', name: 'daemon ops card (status read / resident probe map / guidance commands / zero-install boundary, real modules)', strip: true},
  {file: 'theme-bubble-contrast.mjs', name: 'user bubble contrast sentinel (starship fix / default zero-regression, source mirror)'},
  {file: 'pet-default-off.mjs', name: 'pet default off (pref gate / leeway gate / settings switch, source mirror)'},
  {file: 'switch-scope-copy.mjs', name: 'file-write gate full-scope copy (capdef desc / env note / user manual, source mirror)'},
  {file: 'switch-power-audit.mjs', name: 'capability switch power audit table (filed rows / dispositions, source mirror)'},
  {file: 'pending-approval-marker.mjs', name: 'pending approval card marker chain (tool-generic 等待批准 frame, source mirror)'},
  {file: 'session-preset-persistence.mjs', name: 'session approval tier persistence (select→persist→read back / reopen restore / new-session default, source mirror)'},
  {file: 'user-impression.mjs', name: 'user impression agent settings column (per-persona ledger / self-reference block / summarize seam, real module)', strip: true},
  {file: 'starport-registry.mjs', name: 'starport contribution registry (data-first / injection-field ban / owner lifecycle, real module)', strip: true},
  {file: 'starport-shell.mjs', name: 'starport shell (view lifecycle / single-instance / overlay pairing / async settle, real modules)', strip: true},
  {file: 'pet-agent-status.mjs', name: 'pet agent status machine (five-state vocabulary / live-source table / decay discipline, real module)', strip: true},
  {file: 'pet-approval-quick.mjs', name: 'pet approval quick panel (frame truth source / authoritative match / dual-sync clear, real module)', strip: true},
  {file: 'starport-faces.mjs', name: 'starport faces ledger (17-face honest status vocabulary / note discipline, real module)', strip: true},
  {file: 'starport-browse.mjs', name: 'starport browse gate (protocol whitelist / host policy / fetch-to-text / history, real module)', strip: true},
  {file: 'starport-chart.mjs', name: 'starport chart geometry (data-driven line/bar SVG / degenerate-safe, real module)', strip: true},
  {file: 'starport-organize.mjs', name: 'starport organize (groups / shortcuts CRUD / cross-group move, real module)', strip: true},
  {file: 'starport-lifeflow.mjs', name: 'starport lifeflow (source registry / item stream / unread meter, real module)', strip: true},
  {file: 'starport-notes.mjs', name: 'starport notes (CRUD / pinned ordering / strict validation / persistence round-trip, real modules)', strip: true},
  {file: 'starport-text-editor.mjs', name: 'starport text editor (session CRUD / expected-checked replace / find hit table / mount-cleanup chain, real modules)', strip: true},
  {file: 'starport-doc-pipeline.mjs', name: 'starport doc pipeline (source-preserving segments / range transaction / segment integrity, real modules)', strip: true},
  {file: 'starport-mail.mjs', name: 'starport mail (eml round-trip / header-injection guard / status machine / mailbox, real module)', strip: true},
  {file: 'starport-memory-flow.mjs', name: 'starport memory flow (observation ledger / daily-window compression / term recall ranking / dispatch content, real module)', strip: true},
  {file: 'starport-voice.mjs', name: 'starport voice (state machine full table / level math / voice ledger / duration format, real modules)', strip: true},
  {file: 'starport-db.mjs', name: 'starport db (read-only sql gate / csv escaping / boundary rejection, real module)', strip: true},
  {file: 'starport-folder-tiles.mjs', name: 'starport folder tiles (group CRUD / deterministic ordering / listing mapping, real module)', strip: true},
  {file: 'starport-face-apps.mjs', name: 'starport face apps source mirror (browse gate wiring / real model channel / factory+installer wiring)'},
  {file: 'gateway-auth-face.mjs', name: 'gateway auth face (three-source token / bearer on every fetch / stale credential head purge, real modules)', strip: true},
  {file: 'sse-fetch-stream.mjs', name: 'sse fetch stream (frame parsing / stream lifecycle / token-gated subscription chains, real modules)', strip: true},
  {file: 'user-admin.mjs', name: 'user admin card (list / read-only state / CLI guidance / owner mutations / domain section, real modules)', strip: true},
  {file: 'skill-shelf.mjs', name: 'skill shelf (list / expand / L2 envelope warning head / honest no-face seam, real module)', strip: true},
  {file: 'capture-review.mjs', name: 'capture review card (recent-50 ledger / search filter / same-source review, real modules)', strip: true},
  {file: 'care-card.mjs', name: 'care card (rules / disturbance discipline / heartbeat two columns / honest no-face / read-only guidance, real module)', strip: true},
  {file: 'trait-card.mjs', name: 'trait card (four sliders / lock state / weekly report projection / CLI guidance, real modules)', strip: true},
  {file: 'proposal-card.mjs', name: 'proposal card (proposal ledger IM-card rows / honest no-face / auth seam / read-only, real modules)', strip: true},
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
