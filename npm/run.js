#!/usr/bin/env node
/**
 * CodexCTL Runner - Downloads binary on first run if not present
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const { spawn } = require('child_process');

'use strict';
const VERSION = require('./package.json').version;

const PLATFORMS = {
  'linux-x64': { url: 'codexctl-x86_64-unknown-linux-gnu.tar.gz', ext: '.tar.gz' },
  'linux-arm64': { url: 'codexctl-aarch64-unknown-linux-gnu.tar.gz', ext: '.tar.gz' },
  'darwin-x64': { url: 'codexctl-x86_64-apple-darwin.tar.gz', ext: '.tar.gz' },
  'darwin-arm64': { url: 'codexctl-aarch64-apple-darwin.tar.gz', ext: '.tar.gz' },
  'win32-x64': { url: 'codexctl-x86_64-pc-windows-msvc.zip', ext: '.zip' }
};

function getPlatform() {
  const key = `${os.platform()}-${os.arch()}`;
  return PLATFORMS[key] || PLATFORMS['linux-x64'];
}

const binDir = path.join(__dirname, 'bin');
const platform = getPlatform();
const binName = platform.ext === '.zip' ? 'codexctl.exe' : 'codexctl';
const binPath = path.join(binDir, binName);

async function download() {
  if (!fs.existsSync(binDir)) fs.mkdirSync(binDir, { recursive: true });
  if (fs.existsSync(binPath)) return;
  
  const https = require('https');
  const url = `https://github.com/repohelper/codexctl/releases/download/v${VERSION}/${platform.url}`;
  
  console.log(`Downloading codexctl ${VERSION} for ${os.platform()}-${os.arch()}...`);
  
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        download(res.headers.location).then(resolve).catch(reject);
        return;
      }
      const file = fs.createWriteStream(path.join(binDir, 'download' + platform.ext));
      res.pipe(file);
      file.on('finish', () => {
        if (platform.ext === '.zip') {
          const { execSync } = require('child_process');
          execSync(`powershell -Command "Expand-Archive -Path '${file.path}' -DestinationPath '${binDir}' -Force"`);
        } else {
          const { execSync } = require('child_process');
          execSync(`tar -xzf '${file.path}' -C '${binDir}'`);
        }
        fs.unlinkSync(file.path);
        resolve();
      });
    }).on('error', reject);
  });
}

async function main() {
  await download();
  
  if (os.platform() !== 'win32') {
    try { fs.chmodSync(binPath, 0o755); } catch {}
  }
  
  const child = spawn(binPath, process.argv.slice(2), { stdio: 'inherit', windowsHide: true });
  child.on('exit', (code) => process.exit(code ?? 0));
}

main().catch((err) => { console.error(err.message); process.exit(1); });