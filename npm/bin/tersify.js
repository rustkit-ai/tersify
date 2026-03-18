#!/usr/bin/env node
// Thin wrapper — forwards all arguments to the tersify binary.

const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const binName = process.platform === 'win32' ? 'tersify.exe' : 'tersify';
const localBin = path.join(__dirname, binName);

const binary = fs.existsSync(localBin) ? localBin : 'tersify';

const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  if (result.error.code === 'ENOENT') {
    console.error('[tersify] Binary not found. Run: npm install tersify');
  } else {
    console.error(`[tersify] Error: ${result.error.message}`);
  }
  process.exit(1);
}

process.exit(result.status ?? 0);
