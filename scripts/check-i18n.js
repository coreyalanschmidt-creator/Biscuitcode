#!/usr/bin/env node
// scripts/check-i18n.js
//
// Phase 2 i18n lint gate.
//
// Scans src/ for all static t('key') calls and verifies every key resolves
// in src/locales/en.json. Exits 0 if all keys are present; exits 1 and
// prints missing keys if any are absent.
//
// Dynamic keys like t(variable) are intentionally excluded from this check
// (static analysis cannot verify them). Their resolved values ARE present
// in en.json — the runtime i18n init throws on missing keys in dev mode.
//
// NOTE: i18next-parser 9.x was evaluated but uses --fail-on-update (not
// --fail-on-untranslated-strings which doesn't exist), rewrites en.json
// formatting, and picks up t() patterns from comments. This custom script
// avoids those false positives while satisfying the phase 2 AC intent.

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');

// Load the English locale bundle.
const enJson = JSON.parse(
  readFileSync(join(ROOT, 'src', 'locales', 'en.json'), 'utf8')
);

// Flatten the nested JSON to a set of dot-separated keys.
function flatten(obj, prefix = '') {
  const keys = new Set();
  for (const [k, v] of Object.entries(obj)) {
    if (k === '_meta') continue; // skip metadata section
    const full = prefix ? `${prefix}.${k}` : k;
    if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
      for (const sub of flatten(v, full)) keys.add(sub);
    } else {
      keys.add(full);
    }
  }
  return keys;
}

const definedKeys = flatten(enJson);

// Walk src/ collecting .ts and .tsx files.
function walk(dir) {
  const files = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      files.push(...walk(full));
    } else if (['.ts', '.tsx'].includes(extname(full))) {
      files.push(full);
    }
  }
  return files;
}

const srcFiles = walk(join(ROOT, 'src'));

// Regex: match t('key') or t("key") — static string literals only.
// Excludes dynamic t(variable) patterns intentionally.
// Excludes matches inside line comments (// ...).
const T_STATIC = /\bt\(\s*['"]([^'"]+)['"]/g;

const missingKeys = [];
let totalFound = 0;

for (const file of srcFiles) {
  const src = readFileSync(file, 'utf8');
  // Strip single-line comments to avoid picking up t('key') in comments.
  const stripped = src.replace(/\/\/[^\n]*/g, '');
  for (const match of stripped.matchAll(T_STATIC)) {
    const key = match[1];
    totalFound++;
    if (!definedKeys.has(key)) {
      missingKeys.push({ file: file.replace(ROOT + '/', ''), key });
    }
  }
}

if (missingKeys.length > 0) {
  console.error(`\ni18n check FAILED: ${missingKeys.length} key(s) used in source but missing from src/locales/en.json:\n`);
  for (const { file, key } of missingKeys) {
    console.error(`  ${file}: t('${key}')`);
  }
  process.exit(1);
} else {
  console.log(`i18n check passed: ${totalFound} static key(s) verified against src/locales/en.json`);
  process.exit(0);
}
