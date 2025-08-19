# Pre-commit Configuration for rust-yaml

This document describes the enterprise-grade pre-commit configuration for the rust-yaml project, designed to ensure code quality, security, and consistency across all commits.

## Overview

Our pre-commit configuration implements a comprehensive set of checks that run automatically before each commit, ensuring:

- **Code Quality**: Rust formatting, linting, and compilation checks
- **Security**: Vulnerability scanning, secret detection, and dependency auditing  
- **Standards**: Conventional commit messages, documentation, and consistency
- **Compliance**: License checking and enterprise security policies

## Quick Start

### Installation

```bash
# Install pre-commit (macOS)
brew install pre-commit

# Install pre-commit (other systems)
pip install pre-commit

# Set up the development environment (includes pre-commit setup)
make setup

# Or install pre-commit hooks manually
make pre-commit-install
```

### Basic Usage

```bash
# Run all pre-commit hooks on all files
make pre-commit-run

# Update hooks to latest versions
make pre-commit-update

# Clean pre-commit cache if needed
make pre-commit-clean
```

## Hook Categories

### 1. General Code Quality and Standards

**Purpose**: Ensure consistent file handling, prevent common mistakes, and maintain repository hygiene.

- **File Size Limits**: Prevents files larger than 1MB from being committed
- **Merge Conflict Detection**: Catches unresolved merge conflicts
- **File Format Validation**: Validates TOML, YAML, and JSON syntax
- **Text Consistency**: Fixes line endings, trailing whitespace, and EOF issues
- **Branch Protection**: Prevents direct commits to protected branches

### 2. Rust-Specific Quality Checks

**Purpose**: Enforce Rust code quality and compilation standards.

- **rustfmt**: Automatic code formatting using standard Rust style
- **clippy**: Comprehensive linting with strict enterprise rules including:
  - `-D warnings`: Treat all warnings as errors
  - `-D clippy::all`: Enable all clippy lints
  - `-D clippy::pedantic`: Enable pedantic lints for extra quality
  - `-W clippy::nursery`: Warn on experimental lints
- **Compilation Check**: Ensures code compiles successfully
- **Test Execution**: Runs full test suite before commit
- **Documentation**: Verifies documentation builds correctly

### 3. Security and Vulnerability Management

**Purpose**: Prevent security vulnerabilities and credential leaks.

- **Rust Security Audit**: Scans dependencies for known vulnerabilities using `cargo-audit`
- **Dependency Policy**: Enforces security policies via `cargo-deny` and `deny.toml`
- **Secret Detection**: Uses `detect-secrets` to find leaked credentials
- **Private Key Detection**: Identifies accidentally committed private keys
- **AWS Credential Detection**: Catches AWS access keys and secrets

### 4. Dependency and License Management

**Purpose**: Ensure supply chain security and license compliance.

- **Cargo.lock Verification**: Ensures dependency lock file is up to date
- **Unused Dependencies**: Identifies and removes unused dependencies via `cargo-machete`
- **License Compliance**: Enforces enterprise-approved licenses only
- **Source Validation**: Restricts dependencies to trusted sources

### 5. Documentation and Standards

**Purpose**: Maintain high-quality documentation and consistent formatting.

- **Markdown Linting**: Uses markdownlint with enterprise-friendly rules
- **Custom Markdown Checks**: Our integrated checker for heading/list formatting
- **Conventional Commits**: Enforces conventional commit message format
- **TOML Formatting**: Sorts and formats configuration files

### 6. Infrastructure and Configuration

**Purpose**: Validate configuration files and infrastructure-as-code.

- **GitHub Actions Validation**: Ensures workflow syntax is correct
- **Configuration File Validation**: Checks TOML, YAML, and JSON files
- **Enterprise Security Scanning**: Advanced threat detection

## Configuration Files

### .pre-commit-config.yaml

Main configuration file defining all hooks, their versions, and execution parameters. Key features:

- **Fail Fast**: Stops on first failure for quick feedback
- **Performance Optimized**: Resource-intensive hooks can be skipped in CI
- **Version Controlled**: All hook versions are pinned for reproducibility
- **Comprehensive Coverage**: 20+ different types of checks

### deny.toml

Enterprise security policy configuration for Rust dependencies:

- **License Allowlist**: Only MIT, Apache-2.0, BSD, and other enterprise-friendly licenses
- **Security Policies**: Automatic vulnerability detection and blocking
- **Supply Chain Security**: Restricts dependencies to trusted sources
- **Version Management**: Prevents multiple versions of same dependency

### .secrets.baseline

Baseline configuration for secret detection, preventing false positives while maintaining security.

## Enterprise Features

### Security-First Approach

- **Zero-Trust Dependencies**: All dependencies must pass security screening
- **Vulnerability Blocking**: Any known CVE in dependencies blocks commits
- **Secret Prevention**: Multiple layers of credential leak prevention
- **License Compliance**: Automatic license policy enforcement

### Performance Optimization

- **Selective Execution**: Resource-intensive checks can be disabled for faster feedback
- **Intelligent Caching**: Pre-commit caches results for unchanged files
- **Parallel Execution**: Multiple hooks run concurrently when possible
- **CI Integration**: Optimized configuration for CI/CD environments

### Developer Experience

- **Clear Error Messages**: Detailed feedback on what needs to be fixed
- **Auto-fixing**: Many issues are automatically resolved when possible
- **Integration**: Works seamlessly with existing development workflows
- **Documentation**: Comprehensive guides and examples

## Troubleshooting

### Common Issues

#### Hook Installation Fails

```bash
# Clean and reinstall
make pre-commit-clean
make pre-commit-install
```

#### Rust Tools Not Found

```bash
# Ensure Rust components are installed
rustup component add rustfmt clippy
cargo install cargo-audit cargo-deny cargo-machete
```

#### Secret Detection False Positives

Edit `.secrets.baseline` to allowlist legitimate secrets:

```bash
# Generate new baseline
detect-secrets scan --baseline .secrets.baseline
```

#### AWS Credentials Hook Disabled

The AWS credentials detection hook is disabled by default for Rust projects. Enable it if your project uses AWS services by uncommenting the line in `.pre-commit-config.yaml`.

#### Performance Issues

For faster commits during development, you can skip resource-intensive hooks:

```bash
# Skip tests and coverage for quick commits
SKIP=rust-test,rust-coverage git commit -m "quick fix"

# Run only basic formatting and security checks
SKIP=rust-clippy,rust-test,rust-audit git commit -m "quick fix"
```

Resource-intensive hooks:

- `rust-coverage`: Only runs when manually requested (`pre-commit run --hook-stage manual`)
- `rust-audit`: Only runs when Cargo files change
- `rust-deny`: Only runs when dependency policy files change

### Advanced Configuration

#### Custom Hook Stages

Hooks can be configured to run at different stages:

- `commit`: Run on every commit (default)
- `push`: Run only on push
- `manual`: Run only when explicitly requested

#### Enterprise Customization

The configuration can be extended with:

- Custom security scanners
- Additional license policies  
- Organization-specific rules
- Integration with enterprise security tools

## Best Practices

### For Developers

1. **Run Hooks Regularly**: Use `make pre-commit-run` to test changes
2. **Keep Dependencies Updated**: Regular `cargo audit` and updates
3. **Review Hook Output**: Don't ignore warnings or suggestions
4. **Use Conventional Commits**: Follow the established format

### For Teams

1. **Consistent Tooling**: Ensure all developers use same hook versions
2. **Policy Updates**: Regularly review and update security policies
3. **Training**: Educate team on security and quality standards
4. **Monitoring**: Track hook effectiveness and developer experience

### For Enterprise

1. **Centralized Policies**: Maintain organization-wide configurations
2. **Compliance Reporting**: Monitor license and security compliance
3. **Tool Integration**: Connect with enterprise security and governance tools
4. **Regular Audits**: Periodic review of security policies and effectiveness

## Integration with CI/CD

The pre-commit configuration is designed to work seamlessly with CI/CD:

- **GitHub Actions**: Hooks run automatically on pull requests
- **Performance Tuning**: Resource-intensive hooks are skipped in CI
- **Fail Fast**: Quick feedback on issues
- **Comprehensive Coverage**: Full security and quality checking

## Support and Maintenance

### Updating Hooks

```bash
# Update all hooks to latest versions
make pre-commit-update

# Test updated configuration
make pre-commit-run
```

### Adding New Hooks

1. Edit `.pre-commit-config.yaml`
2. Add hook configuration
3. Test thoroughly
4. Update this documentation

### Policy Changes

1. Review `deny.toml` for security policies
2. Update license allowlists as needed
3. Coordinate with enterprise security team
4. Test against existing codebase

---

This pre-commit configuration represents enterprise-grade standards for Rust development, balancing security, quality, and developer productivity. Regular maintenance and updates ensure it continues to provide value as the project evolves.
