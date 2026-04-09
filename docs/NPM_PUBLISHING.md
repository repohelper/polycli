# npm Publishing Setup Guide

## Overview

CodexCTL uses **npm Trusted Publishers** (via GitHub Actions OIDC) for secure, automated publishing. No long-lived npm tokens stored in GitHub secrets.

## Setup Steps

### 1. Create npm Account (if needed)

1. Go to https://www.npmjs.com/signup
2. Create account: `BhanuKorthiwada` (or your preferred username)
3. Enable 2FA on your account (recommended)

### 2. Create the Package on npm

First, manually publish once to claim the package name:

```bash
cd ~/codexo-public/npm

# Login to npm
npm login

# Publish (will fail if name taken, succeeds if available)
npm publish --access public
```

### 3. Configure Trusted Publisher (One-time setup)

1. Go to https://www.npmjs.com/package/codexctl/access
2. Click **"Add Integration"** (Trusted Publishers section)
3. Select **"GitHub Actions"**
4. Configure:
   - **Repository**: `repohelper/codexctl`
   - **Workflow**: `npm-publish.yml`
   - **Environment**: (leave empty for any)
5. Click **"Add Integration"**

### 4. Create GitHub Repository Secret

Even with trusted publishing, you need a temporary token for the initial setup:

1. Go to https://www.npmjs.com/settings/BhanuKorthiwada/tokens
2. Create **Granular Access Token**:
   - Name: `GitHub Actions Publish`
   - Packages: `codexctl` (Read and Write)
   - Expiration: 90 days
3. Copy the token

4. Go to GitHub repo → Settings → Secrets and variables → Actions
5. Create **New repository secret**:
   - Name: `NPM_TOKEN`
   - Value: (paste your npm token)

### 5. Test the Setup

Trigger a release:

```bash
# Create and push a tag
git tag v0.1.0
git push origin v0.1.0
```

This will:
1. Trigger `release.yml` - Build and create GitHub Release
2. Trigger `npm-publish.yml` - Publish to npm with provenance

### 6. Verify Trusted Publishing Works

Check the npm package page: https://www.npmjs.com/package/codexctl

You should see:
- ✅ "Provenance" badge (linked GitHub Actions run)
- ✅ "Trusted Publisher" information

## How Trusted Publishing Works

```
GitHub Actions (OIDC Token)
        ↓
npm Registry (verifies signature)
        ↓
Package Published with Provenance
```

**Benefits:**
- No long-lived tokens in GitHub secrets
- Every publish is cryptographically linked to GitHub Actions run
- Users can verify package was built from your source code
- Automatic token rotation

## Troubleshooting

### "404 Not Found" on first publish
The package doesn't exist yet. Run manual publish once:
```bash
cd npm && npm login && npm publish --access public
```

### "Unauthorized" error
Check that:
1. `NPM_TOKEN` secret is set correctly
2. Token has write access to `codexctl` package
3. Trusted publisher is configured for correct workflow file

### Provenance not showing
Ensure workflow has:
```yaml
permissions:
  id-token: write
```
And publish command includes `--provenance` flag.

## Maintenance

### Rotating Tokens
Trusted publishing doesn't require token rotation, but if you used legacy tokens:
1. Generate new token on npm
2. Update `NPM_TOKEN` secret in GitHub
3. Revoke old token on npm

### Adding Collaborators
1. Go to https://www.npmjs.com/package/codexctl/access
2. Click **"Add Member"**
3. Enter npm username and select role (Read/Write/Admin)

## Resources

- npm Trusted Publishers: https://docs.npmjs.com/trusted-publishers
- Provenance: https://docs.npmjs.com/generating-provenance-statements
- GitHub OIDC: https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/about-security-hardening-with-openid-connect
