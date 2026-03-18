#!/usr/bin/env node
// Postinstall: download the tersify binary for the current platform.

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const { createGunzip } = require('zlib');

const VERSION = require('../package.json').version;
const REPO = 'rustkit-ai/tersify';
const BIN_DIR = path.join(__dirname, '..', 'bin');
const BIN_PATH = path.join(BIN_DIR, process.platform === 'win32' ? 'tersify.exe' : 'tersify');

// Skip download if binary already exists (e.g. during development)
if (fs.existsSync(BIN_PATH)) {
  process.exit(0);
}

const TARGET_MAP = {
  'darwin-arm64':  'aarch64-apple-darwin',
  'darwin-x64':    'x86_64-apple-darwin',
  'linux-arm64':   'aarch64-unknown-linux-musl',
  'linux-x64':     'x86_64-unknown-linux-musl',
  'win32-x64':     'x86_64-pc-windows-msvc',
};

const platform = `${process.platform}-${process.arch}`;
const target = TARGET_MAP[platform];

if (!target) {
  console.warn(`[tersify] No pre-built binary for ${platform}. Install via: cargo install tersify`);
  process.exit(0);
}

const ext = process.platform === 'win32' ? '.zip' : '.tar.gz';
const archive = `tersify-${target}${ext}`;
const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${archive}`;
const tmpArchive = path.join(BIN_DIR, archive);

console.log(`[tersify] Downloading ${archive}...`);

fs.mkdirSync(BIN_DIR, { recursive: true });

function download(url, dest, cb) {
  const file = fs.createWriteStream(dest);
  https.get(url, (res) => {
    // Follow redirects
    if (res.statusCode === 301 || res.statusCode === 302) {
      file.close();
      fs.unlinkSync(dest);
      return download(res.headers.location, dest, cb);
    }
    if (res.statusCode !== 200) {
      file.close();
      return cb(new Error(`HTTP ${res.statusCode} for ${url}`));
    }
    res.pipe(file);
    file.on('finish', () => file.close(cb));
  }).on('error', (err) => {
    fs.unlink(dest, () => {});
    cb(err);
  });
}

download(url, tmpArchive, (err) => {
  if (err) {
    console.warn(`[tersify] Download failed: ${err.message}`);
    console.warn('[tersify] Install manually: cargo install tersify');
    process.exit(0);
  }

  try {
    if (process.platform === 'win32') {
      execSync(`powershell -Command "Expand-Archive -Path '${tmpArchive}' -DestinationPath '${BIN_DIR}' -Force"`);
    } else {
      execSync(`tar -xzf "${tmpArchive}" -C "${BIN_DIR}" tersify`);
      fs.chmodSync(BIN_PATH, 0o755);
    }
    fs.unlinkSync(tmpArchive);
    console.log(`[tersify] Installed to ${BIN_PATH}`);
  } catch (e) {
    console.warn(`[tersify] Extraction failed: ${e.message}`);
    process.exit(0);
  }
});
