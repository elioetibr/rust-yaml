# Version Management Guide

This project uses [GitVersion](https://gitversion.net/) for semantic versioning and automated release management.

## Overview

- **GitVersion 6.x** is used for semantic version calculation
- **GitHub Actions** handle automated releases and version bumping
- **Semantic versioning** follows the [SemVer](https://semver.org/) specification
- **Commit messages** can control version bumping using special markers

## Version Calculation

GitVersion analyzes the Git history and branch structure to calculate versions automatically:

- **Main branch**: Release versions (e.g., `1.2.3`)
- **Feature branches**: Pre-release versions (e.g., `1.2.4-feature-branch.1`)
- **Pull request branches**: Pre-release versions (e.g., `1.2.4-PullRequest.123`)

## Commit Message Conventions

Use these markers in commit messages to control version bumping:

- `+semver: major` or `+semver: breaking` - Increment major version
- `+semver: minor` or `+semver: feature` - Increment minor version  
- `+semver: patch` or `+semver: fix` - Increment patch version
- `+semver: none` or `+semver: skip` - No version increment

### Examples

```bash
# Major version bump
git commit -m "feat: redesign API interface

BREAKING CHANGE: The API interface has been completely redesigned.

+semver: major"

# Minor version bump  
git commit -m "feat: add new YAML parsing features

+semver: minor"

# Patch version bump
git commit -m "fix: resolve parsing edge case

+semver: patch"

# No version bump
git commit -m "docs: update README

+semver: none"
```

## Release Process

### Option 1: Automated Script (Recommended)

Use the provided release script:

```bash
# Interactive release (prompts for version type)
./scripts/release.sh

# Direct version specification
./scripts/release.sh 1.2.3
```

The script will:

1. Verify you're on the main branch
2. Check for a clean working directory
3. Update version in `Cargo.toml` and `GitVersion.yml`
4. Run tests and checks
5. Commit changes and create/push tags
6. Trigger GitHub Actions for release

### Option 2: Manual Process

1. **Update version files**:

   ```bash
   # Update Cargo.toml
   sed -i 's/^version = ".*"/version = "1.2.3"/' Cargo.toml

   # Update GitVersion.yml
   sed -i 's/^next-version: .*/next-version: 1.2.3/' GitVersion.yml

   # Update lockfile
   cargo update -p rust-yaml
   ```

2. **Run validations**:

   ```bash
   cargo test
   cargo clippy --all-targets --all-features -- -D warnings
   cargo build --release
   ```

3. **Commit and tag**:

   ```bash
   git add Cargo.toml GitVersion.yml Cargo.lock
   git commit -m "chore: release version 1.2.3

   +semver: none"

   git tag v1.2.3
   git push origin main
   git push origin v1.2.3
   ```

### Option 3: GitHub Actions Manual Trigger

Use the manual version bump workflow:

1. Go to **Actions** → **Manual Version Bump**
2. Click **Run workflow**
3. Select version bump type or enter custom version
4. Click **Run workflow**

## GitHub Actions Workflows

### CI Workflow (`.github/workflows/ci.yml`)

- Runs on every push and PR
- Calculates and displays version information using GitVersion
- Runs tests across multiple Rust versions and platforms

### Release Workflow (`.github/workflows/release.yml`)

- Triggered by version tags (e.g., `v1.2.3`)
- Validates version consistency
- Builds and tests the release
- Creates GitHub release with changelog
- Publishes to crates.io

### Version Bump Workflow (`.github/workflows/version-bump.yml`)

- Automatically runs after successful releases
- Bumps to next development version
- Updates `Cargo.toml` and `GitVersion.yml`

### Manual Version Bump Workflow (`.github/workflows/manual-version-bump.yml`)

- Manually triggered for version management
- Supports major, minor, patch, or custom versions
- Commits and pushes version changes

## Configuration Files

### `GitVersion.yml`

Contains GitVersion configuration:

- Branch patterns and versioning strategies
- Version increment rules
- Tag and branch handling

### `Cargo.toml`

Contains the current package version that gets updated for releases.

## Development Workflow

### Feature Development

1. Create feature branch: `git checkout -b feature/my-feature`
2. Develop and commit changes
3. Push branch and create PR
4. CI builds with pre-release version (e.g., `1.2.4-feature-my-feature.1`)

### Bug Fixes

1. Create fix branch: `git checkout -b fix/bug-description`
2. Fix issue and commit with appropriate semver marker
3. Create PR for review
4. Merge to main triggers version calculation

### Releases

1. Ensure all features are merged to main
2. Use release script or manual process
3. Tag triggers automated release workflow
4. Post-release version bump happens automatically

## Troubleshooting

### Version Mismatch Errors

If GitVersion calculation doesn't match expectations:

1. Check commit message semver markers
2. Verify branch is up to date
3. Review `GitVersion.yml` configuration

### Failed Releases

If release workflow fails:

1. Check GitHub Actions logs
2. Verify `CARGO_REGISTRY_TOKEN` secret is set
3. Ensure version doesn't already exist on crates.io

### Manual Recovery

To manually fix version issues:

1. Delete problematic tags: `git tag -d v1.2.3 && git push origin :refs/tags/v1.2.3`
2. Reset version in files
3. Re-run release process

## Best Practices

1. **Use meaningful commit messages** with appropriate semver markers
2. **Test thoroughly** before releases
3. **Keep CHANGELOG.md updated** for user-facing changes
4. **Use feature branches** for development
5. **Review version increments** in CI to catch issues early
6. **Coordinate releases** with team members
7. **Monitor release workflows** for successful completion

## References

- [GitVersion Documentation](https://gitversion.net/)
- [Semantic Versioning](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Cargo Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
