#!/usr/bin/env node
// 素材许可门禁（avatar-pack-admission.md §1b/§4 的机器执行）。
//
// 规则：
//   1. 扫描面内（public/、src/assets/、avatar-packs/）每个素材文件必须在
//      asset-licenses.json 台账有行——无行即红（杜绝"先用后换"漏进发布物）；
//   2. 台账行指向的文件必须存在——陈旧行即红（零债纪律）；
//   3. origin=third-party 必须：许可在白名单 + 署名非空 + 来源 http(s) URL；
//      白名单外许可（非商用/禁改/禁二销/不明）一律红；
//   4. origin ∈ first-party / third-party / procedural 之外的值红（命名即承诺）。
//
// 用法：node scripts/check-asset-licenses.mjs（frontend/companion-desktop/ 下）
// 退出码：0 = 干净；1 = 有违规（逐条打印）。
import {readFileSync, readdirSync, statSync} from 'node:fs';
import {dirname, join, relative, sep} from 'node:path';
import {fileURLToPath} from 'node:url';

const pkgRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const LEDGER = join(pkgRoot, 'asset-licenses.json');
const SCAN_ROOTS = ['public', 'src/assets', 'avatar-packs'];
const ORIGINS = new Set(['first-party', 'third-party', 'procedural']);

const errors = [];

function walk(rootAbs) {
  let entries;
  try {
    entries = readdirSync(rootAbs, {withFileTypes: true});
  } catch {
    return []; // 目录不存在（如 avatar-packs 尚未立）= 无可扫，不是错
  }
  const files = [];
  for (const entry of entries) {
    const abs = join(rootAbs, entry.name);
    if (entry.isDirectory()) files.push(...walk(abs));
    else if (entry.isFile()) files.push(abs);
  }
  return files;
}

function isHttpUrl(value) {
  return /^https?:\/\/\S+$/.test(value);
}

// ---- 读台账 ----
let ledger;
try {
  ledger = JSON.parse(readFileSync(LEDGER, 'utf8'));
} catch (caught) {
  console.error(`❌ 读不到台账 ${LEDGER}: ${caught.message}`);
  process.exit(1);
}
const whitelist = new Set(
  Array.isArray(ledger.thirdPartyLicenseWhitelist) ? ledger.thirdPartyLicenseWhitelist : [],
);
const rows = Array.isArray(ledger.assets) ? ledger.assets : [];
const byPath = new Map();
for (const row of rows) {
  if (!row || typeof row.path !== 'string' || !row.path) {
    errors.push(`台账行缺 path: ${JSON.stringify(row)}`);
    continue;
  }
  if (byPath.has(row.path)) errors.push(`台账行重复: ${row.path}`);
  byPath.set(row.path, row);
}

// ---- 正向：扫描面内每个文件必须有行 ----
const scanned = new Set();
for (const root of SCAN_ROOTS) {
  for (const abs of walk(join(pkgRoot, root))) {
    const rel = relative(pkgRoot, abs).split(sep).join('/');
    scanned.add(rel);
    const row = byPath.get(rel);
    if (!row) {
      errors.push(`素材无台账行（无行即红）: ${rel}`);
      continue;
    }
    if (!ORIGINS.has(row.origin)) {
      errors.push(`origin 非法（${row.origin}）: ${rel}（只许 first-party / third-party / procedural）`);
      continue;
    }
    if (row.origin === 'third-party') {
      if (!whitelist.has(row.license)) {
        errors.push(`第三方许可不在白名单（${row.license}）: ${rel}`);
      }
      if (typeof row.attribution !== 'string' || !row.attribution.trim()) {
        errors.push(`第三方素材缺署名文案: ${rel}`);
      }
      if (typeof row.source !== 'string' || !isHttpUrl(row.source)) {
        errors.push(`第三方素材缺来源 URL: ${rel}`);
      }
    } else if (typeof row.license !== 'string' || !row.license.trim()) {
      errors.push(`台账行缺 license: ${rel}`);
    }
  }
}

// ---- 反向：台账行必须指向存在的文件（陈旧行 = 债） ----
for (const path of byPath.keys()) {
  if (!scanned.has(path)) {
    try {
      statSync(join(pkgRoot, path));
    } catch {
      errors.push(`台账行指向不存在的文件（陈旧行）: ${path}`);
    }
  }
}

if (errors.length > 0) {
  console.error(`❌ 素材许可门禁: ${errors.length} 处违规`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}
console.log(`✅ 素材许可门禁: ${scanned.size} 个素材全在台账，第三方许可全在白名单`);
