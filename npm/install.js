#!/usr/bin/env node
/**
 * CodexCTL Installation Script
 * Downloads the appropriate binary for the current platform from GitHub Releases
 */

const https = require('https');
const http = require('http');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync, spawn } = require('child_process');

const VERSION = require('./package.json').version;

const platforms = {
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'win32-x64': 'x86_64-pc-windows-msvc'
};

function getPlatform() {
  const platform = os.platform();
  const arch = os.arch();
  const key = `${platform}-${arch}`;
  
  if (!platforms[key]) {
    console.error(`Unsupported platform: ${platform} ${arch}`);
    console.error('Supported platforms:', Object.keys(platforms).join(', '));
    process.exit(1);
  }
  
  return platforms[key];
}

function getDownloadUrl(target) {
  const ext = target.includes('windows') ? 'zip' : 'tar.gz';
  return `https://github.com/repohelper/codexctl/releases/download/v${VERSION}/codexctl-${target}.${ext}`;
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const protocol = url.startsWith('https') ? https : http;
    
    const request = protocol.get(url, { headers: { 'User-Agent': 'codexctl-install' } }, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        const redirectUrl = response.headers.location;
        if (redirectUrl) {
          downloadFile(redirectUrl, dest).then(resolve).catch(reject);
          return;
        }
      }
      
      if (response.statusCode !== 200) {
        reject(new Error(`Download failed with status ${response.statusCode}: ${url}`));
        return;
      }
      
      const file = fs.createWriteStream(dest);
      response.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
    });
    
    request.on('error', reject);
  });
}

function extractArchive(archivePath, destDir) {
  const ext = path.extname(archivePath);
  
  if (ext === '.zip') {
    if (os.platform() === 'win32') {
      execSync(`powershell -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`, { stdio: 'inherit' });
    } else {
      execSync(`unzip -o "${archivePath}" -d "${destDir}"`, { stdio: 'inherit' });
    }
  } else {
    execSync(`tar -xzf "${archivePath}" -C "${destDir}"`, { stdio: 'inherit' });
  }
}

async function install() {
  const binDir = path.join(__dirname, 'bin');
  
  // Skip if binaries already exist
  if (fs.existsSync(path.join(binDir, 'codexctl'))) {
    console.log('CodexCTL binaries already exist, skipping download');
    return;
  }
  
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const target = getPlatform();
  const ext = target.includes('windows') ? 'zip' : 'tar.gz';
  const archivePath = path.join(binDir, `codexctl-${target}.${ext}`);
  
  console.log(`Downloading CodexCTL v${VERSION} for ${target}...`);
  
  const url = getDownloadUrl(target);
  console.log(`URL: ${url}`);
  
  try {
    // First try direct download
    await downloadFile(url, archivePath);
  } catch (err) {
    // If direct fails, try latest tag
    const latestUrl = getDownloadUrl(target).replace(`/v${VERSION}`, '/latest');
    console.log(`Retrying with latest...`);
    await downloadFile(latestUrl, archivePath);
  }
  
  console.log('Extracting...');
  extractArchive(archivePath, binDir);
  
  // Clean up archive
  fs.unlinkSync(archivePath);
  
  // Make executable
  if (os.platform() !== 'win32') {
    try {
      fs.chmodSync(path.join(binDir, 'codexctl'), 0o755);
    } catch (e) {
      // Try cdx if codexctl name differs
      const cdxPath = path.join(binDir, 'cdx');
      if (fs.existsSync(cdxPath)) {
        fs.chmodSync(cdxPath, 0o755);
      }
    }
  }
  
  console.log('CodexCTL installed successfully!');
  console.log('Run: codexctl --help');
}

install().catch(err => {
  console.error('Installation failed:', err.message);
  console.error('');
  console.error('To install manually:');
  console.error(`1. Download from: https://github.com/repohelper/codexctl/releases`);
  console.error(`2. Extract and add to your PATH`);
  process.exit(1);
});