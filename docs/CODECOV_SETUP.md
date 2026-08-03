<!--
SPDX-FileCopyrightText: Rust Yaml contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Codecov Setup Guide for rust-yaml

## Prerequisites

1. **Codecov Account**: Sign up at [codecov.io](https://about.codecov.io/)
2. **Repository Access**: Link your GitHub account to Codecov

## Setup Steps

### 1. Add Repository to Codecov

1. Log in to [Codecov](https://app.codecov.io/)
2. Navigate to "Add New Repository"
3. Find and select `elioetibr/rust-yaml`
4. Follow the setup wizard

### 2. Get Codecov Token

1. In Codecov dashboard, go to your repository settings
2. Copy the upload token (starts with a UUID format)
3. Keep this token secure - it's needed for uploading coverage reports

### 3. Add Token to GitHub Secrets

1. Go to your GitHub repository settings
2. Navigate to Settings → Secrets and variables → Actions
3. Click "New repository secret"
4. Name: `CODECOV_TOKEN`
5. Value: Paste your Codecov token
6. Click "Add secret"

### 4. Verify Configuration

The repository includes:

- `.github/workflows/ci.yml` - CI workflow with coverage job
- `codecov.yml` - Codecov configuration file

### 5. Trigger Coverage Report

Coverage reports are generated on:

- Every push to main/master branch
- Every pull request

To manually trigger:

1. Push a commit to any branch
2. The CI workflow will automatically run
3. Check the "Coverage Report" job in GitHub Actions
4. View results at: <https://app.codecov.io/gh/elioetibr/rust-yaml>

## Local Testing

To generate coverage reports locally:

```bash
# Install cargo-tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --lib --tests --out lcov --output-dir . --verbose

# View the generated lcov.info file
ls -la lcov.info
```

## Troubleshooting

### No Coverage Reports Appearing

1. **Check GitHub Actions**: Ensure the coverage job is running successfully
2. **Verify Token**: Confirm CODECOV_TOKEN is set in GitHub secrets
3. **Check Logs**: Review the "Upload coverage to Codecov" step logs in GitHub Actions
4. **Codecov Status**: Check [Codecov status page](https://status.codecov.io/)

### Low Coverage Percentage

- Add more tests to increase coverage
- Use `cargo tarpaulin --print-summary` to see uncovered lines
- Focus on testing critical paths and error handling

### Coverage Badge Not Updating

1. Clear browser cache
2. Wait a few minutes for Codecov to process
3. Check badge URL format in README.md

## Coverage Goals

`codecov.yml` is the source of truth; these are its settings, not a second
policy:

- **Patch** — every PR is gated on the coverage of the lines it adds or
  changes (`target: auto`, 1% threshold). This is the check contributors act on.
- **Project** — `target: auto` with a 1% threshold, scoped to `src`, so overall
  coverage may not regress by more than a point.
- `range: "70...100"` sets only how the badge and report are coloured. It is
  not a pass/fail threshold.

## Additional Resources

- [Codecov Documentation](https://docs.codecov.io/)
- [cargo-tarpaulin Documentation](https://github.com/xd009642/tarpaulin)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
