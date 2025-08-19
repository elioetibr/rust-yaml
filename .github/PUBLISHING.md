# Publishing rust-yaml

This document describes how to publish new versions of rust-yaml to crates.io and docs.rs.

## Prerequisites

### Required Secrets

Before publishing, ensure the following secrets are configured in your GitHub repository:

1. **`CARGO_REGISTRY_TOKEN`** - Your crates.io API token
   - Get it from: https://crates.io/me (click "API Tokens")
   - Add to GitHub: Settings → Secrets and variables → Actions → New repository secret
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: Your crates.io API token

2. **`GITHUB_TOKEN`** - Automatically provided by GitHub Actions
   - No configuration needed, available by default

### Required Permissions

Ensure GitHub Actions has the following permissions:
- **Contents**: Write (for creating releases and tags)
- **Pages**: Write (for publishing documentation to GitHub Pages)
- **Pull Requests**: Write (for automated PRs if needed)

Go to: Settings → Actions → General → Workflow permissions
- Select "Read and write permissions"

## Publishing Methods

### Method 1: Automatic Publishing on Push to Main

When you push to the main branch with a version change:

1. The CI workflow runs all tests
2. If tests pass, it automatically:
   - Creates a GitHub release
   - Publishes to crates.io
   - Builds and publishes documentation

### Method 2: Manual Publishing via Workflow Dispatch

1. Go to Actions → Publish workflow
2. Click "Run workflow"
3. Enter the version to publish (e.g., `0.1.0`)
4. Optionally check "dry run" to test without publishing
5. Click "Run workflow"

### Method 3: Publishing on GitHub Release

1. Create a new release on GitHub
2. Tag it with `v{version}` (e.g., `v0.1.0`)
3. The publish workflow will automatically:
   - Validate the version
   - Run tests
   - Publish to crates.io
   - Update documentation

## Publishing Process

### 1. Prepare the Release

```bash
# Update version in Cargo.toml
cargo set-version 0.1.0

# Run tests locally
cargo test --all-features
cargo test --no-default-features

# Check the package
cargo package --list
cargo publish --dry-run

# Commit changes
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.1.0"
git push origin main
```

### 2. Create a GitHub Release

```bash
# Create and push a tag
git tag -a v0.1.0 -m "Release version 0.1.0"
git push origin v0.1.0

# Or use GitHub CLI
gh release create v0.1.0 --generate-notes
```

### 3. Monitor the Publishing

1. Check GitHub Actions for the workflow status
2. Once complete, verify:
   - crates.io: https://crates.io/crates/rust-yaml
   - docs.rs: https://docs.rs/rust-yaml
   - GitHub Pages: https://[your-username].github.io/rust-yaml/

## Documentation

### docs.rs

Documentation is automatically built by docs.rs when you publish to crates.io.

Configuration in `Cargo.toml`:
```toml
[package.metadata.docs.rs]
rustdoc-args = [
    "--generate-link-to-definition",
    "--cfg", "docsrs",
    "--document-private-items",
]
all-features = true
```

### GitHub Pages

Documentation is also published to GitHub Pages:

1. Enable GitHub Pages in repository settings:
   - Settings → Pages
   - Source: GitHub Actions

2. Documentation will be available at:
   - https://[your-username].github.io/rust-yaml/

## Version Management

### Semantic Versioning

Follow semantic versioning (https://semver.org/):

- **MAJOR** (1.0.0): Breaking API changes
- **MINOR** (0.1.0): New features, backward compatible
- **PATCH** (0.0.1): Bug fixes, backward compatible

### Pre-release Versions

For pre-releases, use:
- Alpha: `0.1.0-alpha.1`
- Beta: `0.1.0-beta.1`
- Release Candidate: `0.1.0-rc.1`

## Troubleshooting

### Issue: "Version already exists on crates.io"

**Solution**: Increment the version in Cargo.toml. You cannot republish the same version.

### Issue: "Invalid CARGO_REGISTRY_TOKEN"

**Solution**: 
1. Generate a new token at https://crates.io/me
2. Update the secret in GitHub repository settings

### Issue: "Package verification failed"

**Solution**: Run locally to debug:
```bash
cargo package --verbose
cargo publish --dry-run
```

### Issue: "Documentation not appearing on docs.rs"

**Solution**: 
1. Check build status at: https://docs.rs/crate/rust-yaml/builds
2. Ensure your `Cargo.toml` has valid metadata
3. Check for build errors in the docs.rs build log

## Checklist for Publishing

- [ ] All tests pass locally
- [ ] Version bumped in `Cargo.toml`
- [ ] CHANGELOG.md updated
- [ ] Documentation is up to date
- [ ] Examples work with new version
- [ ] Breaking changes documented (if any)
- [ ] `cargo publish --dry-run` succeeds
- [ ] Git tag created and pushed
- [ ] GitHub release created with notes

## Security Considerations

1. **Never commit tokens**: Keep your `CARGO_REGISTRY_TOKEN` secret
2. **Use environments**: Configure GitHub environments for production releases
3. **Review before publishing**: Always review changes before publishing
4. **Test locally first**: Run `cargo publish --dry-run` before actual publishing

## Support

For issues with publishing:
1. Check GitHub Actions logs
2. Review docs.rs build logs
3. Open an issue in the repository
4. Contact crates.io support if needed