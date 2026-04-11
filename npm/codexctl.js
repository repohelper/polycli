#!/usr/bin/env node
/**
 * CodexCTL - Wrapper that uses platform-specific binary from optionalDependencies
 * 
 * When npm installs this package, it automatically downloads the correct
 * @codexctl/{platform} package based on OS/arch. The binary is then available
 * in node_modules/@codexctl/{platform}/bin/
 */

const path = require('path');
const os = require('os');
const { spawn } = require('child_process');

'use strict';

const PLATFORMS = {
  'linux-x64': '@codexctl/linux-x64',
  'linux-arm64': '@codexctl/linux-arm64',
  'darwin-x64': '@codexctl/darwin-x64',
  'darwin-arm64': '@codexctl/darwin-arm64',
  'win32-x64': '@codexctl/win32-x64'
};

function getPlatform() {
  const key = `${os.platform()}-${os.arch()}`;
  return PLATFORMS[key] || PLATFORMS['linux-x64'];
}

const platformPackage = getPlatform();
const binDir = path.join(__dirname, '..', platformPackage, 'bin');
const isWindows = os.platform() === 'win32';
const binaryName = isWindows ? 'codexctl.exe' : 'codexctl';
const binaryPath = path.join(binDir, binaryName);

const child = spawn(binaryPath, process.argv.slice(2), { 
  stdio: 'inherit', 
  windowsHide: true 
});

child.on('exit', (code) => process.exit(code ?? 0));