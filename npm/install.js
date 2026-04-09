#!/usr/bin/env node
/**
 * CodexCTL Installation Script
 * Downloads the appropriate binary for the current platform
 */

const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');

const VERSION = require('./package.json').version;
const BINARY_NAME = 'codexctl';

// Platform mappings
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
    const file = fs.createWriteStream(dest);
    https.get(url, { followRedirects: true }, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        downloadFile(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      
      if (response.statusCode !== 200) {
        reject(new Error(`Download failed with status ${response.statusCode}`));
        return;
      }
      
      response.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', reject);
  });
}

function extractArchive(archivePath, destDir) {
  if (archivePath.endsWith('.zip')) {
    execSync(`unzip -o "${archivePath}" -d "${destDir}"`, { stdio: 'inherit' });
  } else {
    execSync(`tar -xzf "${archivePath}" -C "${destDir}"`, { stdio: 'inherit' });
  }
}

async function install() {
  const binDir = path.join(__dirname, 'bin');
  
  // Skip if binaries already exist (development or manual install)
  if (fs.existsSync(path.join(binDir, BINARY_NAME))) {
    console.log('CodexCTL binaries already exist, skipping download');
    return;
  }
  
  // Ensure bin directory exists
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  
  const target = getPlatform();
  const url = getDownloadUrl(target);
  const ext = target.includes('windows') ? 'zip' : 'tar.gz';
  const archivePath = path.join(binDir, `codexctl-${target}.${ext}`);
  
  console.log(`Downloading CodexCTL v${VERSION} for ${target}...`);
  console.log(`URL: ${url}`);
  
  try {
    await downloadFile(url, archivePath);
    console.log('Download complete, extracting...');
    
    extractArchive(archivePath, binDir);
    fs.unlinkSync(archivePath);
    
    // Make binaries executable on Unix
    if (os.platform() !== 'win32') {
      execSync(`chmod +x "${path.join(binDir, BINARY_NAME)}"`);
      execSync(`chmod +x "${path.join(binDir, 'cdx')}"`);
    }
    
    console.log('CodexCTL installed successfully!');
    console.log('Run: cdx --help');
  } catch (error) {
    console.error('Installation failed:', error.message);
    console.error('You can manually download from: https://github.com/repohelper/codexctl/releases');
    process.exit(1);
  }
}

install().catch(err => {
  console.error('Unexpected error:', err);
  process.exit(1);
});
