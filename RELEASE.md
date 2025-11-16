# 🚀 Release Guide for Sound Stitch

This guide explains how to create and publish releases of Sound Stitch with automatic cross-platform builds.

## 📦 Quick Release (Recommended)

### Option 1: Interactive Release Script
Run the interactive release manager:

**Windows:**
```cmd
scripts\release.bat
```

**macOS/Linux:**
```bash
chmod +x scripts/release.sh
./scripts/release.sh
```

The script will guide you through:
- Choosing release type (patch/minor/major)
- Version validation
- Automatic file updates
- Git tagging and pushing
- GitHub Actions triggering

### Option 2: NPM Scripts
For quick releases using npm:

```bash
# Patch release (1.0.0 → 1.0.1)
npm run release:patch

# Minor release (1.0.0 → 1.1.0) 
npm run release:minor

# Major release (1.0.0 → 2.0.0)
npm run release:major
```

## 🔄 What Happens During Release

1. **Version Update**: All version fields are automatically updated in:
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`

2. **Git Operations**:
   - Changes are committed
   - Version tag is created (e.g., `v1.0.0`)
   - Tag and commits are pushed to GitHub

3. **Automated Builds**: GitHub Actions automatically:
   - Builds for Windows (x64)
   - Builds for macOS (Universal)
   - Builds for Linux (x64)
   - Creates GitHub release with all binaries

## 📥 Release Artifacts

Each release includes these download options:

### Windows
- `Sound-Stitch_v1.0.0_x64-setup.exe` - Setup installer
- `Sound-Stitch_v1.0.0_x64_en-US.msi` - MSI installer

### macOS
- `Sound-Stitch_v1.0.0_universal.dmg` - DMG installer (Universal binary)

### Linux
- `Sound-Stitch_v1.0.0_amd64.deb` - Debian package
- `Sound-Stitch_v1.0.0_amd64.AppImage` - Portable AppImage

## 🛠️ Manual Release Process

If you prefer manual control:

1. **Update Version**:
   ```bash
   npm run version:sync
   ```

2. **Create Tag**:
   ```bash
   git add .
   git commit -m "bump version to 1.0.0"
   git tag -a v1.0.0 -m "Release v1.0.0"
   git push origin main
   git push origin v1.0.0
   ```

3. **Monitor Build**: Visit GitHub Actions tab to monitor the build progress.

## 🎯 Release Types

| Type | When to Use | Version Change |
|------|-------------|----------------|
| **Patch** | Bug fixes, small improvements | 1.0.0 → 1.0.1 |
| **Minor** | New features, backwards compatible | 1.0.0 → 1.1.0 |
| **Major** | Breaking changes, major overhaul | 1.0.0 → 2.0.0 |

## 🔍 Monitoring Releases

- **GitHub Actions**: Monitor build progress at `https://github.com/YOUR_USERNAME/REPO_NAME/actions`
- **Releases Page**: View published releases at `https://github.com/YOUR_USERNAME/REPO_NAME/releases`
- **Build Logs**: Check individual platform builds for any issues

## 🚨 Troubleshooting

### Build Failures
- Check GitHub Actions logs for specific platform failures
- Ensure all dependencies are properly configured
- Verify Tauri configuration is valid

### Version Conflicts
- Make sure all version files are in sync
- Use `npm run version:sync` to synchronize versions

### Missing Artifacts
- Builds may take 10-30 minutes depending on platform
- Check if all required secrets are configured in GitHub

## 🔒 GitHub Configuration

Ensure your GitHub repository has:
- Actions enabled
- Push access to main branch
- No required status checks blocking tag pushes

## 📝 Changelog

The release workflow automatically generates changelogs based on:
- Git commit messages since last release
- Filters out merge commits and version bumps
- Provides meaningful release notes

## 🎉 Success!

Once complete, users can download Sound Stitch for their platform directly from your GitHub releases page. The automated system ensures consistent, professional releases across all supported platforms.

---

**Need help?** Check the GitHub Actions logs or create an issue in the repository.

## 🎯 **Creating Your First Release**

### **Step 1: Check Repository Setup**
First, make sure your GitHub repository is properly configured:

```bash
# Check if you have a remote repository
git remote -v

# If no remote, add one:
# git remote add origin https://github.com/YOUR_USERNAME/YOUR_REPO_NAME.git
```

### **Step 2: Ensure Clean State** 
Make sure all your changes are committed:

```bash
# Check status
git status

# If you have uncommitted changes:
git add .
git commit -m "Prepare for first release"
git push origin main
```

### **Step 3: Create Your First Release**
Since you're at version `0.1.0`, create your first release:

**Option A - Quick Command:**
```bash
npm run release:minor
```

**Option B - Interactive Script:**
```cmd
# Windows
scripts\release.bat

# Mac/Linux  
./scripts/release.sh
```

### **Step 4: Monitor the Build**
1. After running the release command, go to your GitHub repository
2. Click on **"Actions"** tab
3. You'll see the **"Release Build"** workflow running
4. Wait 15-30 minutes for all platforms to build

### **Step 5: Check Your Release**
Once complete:
1. Go to your GitHub repository
2. Click **"Releases"** on the right side
3. You'll see your new release with downloads for:
   - Windows (.exe and .msi)
   - macOS (.dmg)
   - Linux (.deb and .AppImage)

---
