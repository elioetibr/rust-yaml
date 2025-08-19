# Pull Request

## Summary

<!-- Provide a brief summary of your changes -->

## Type of Change

<!-- Mark the relevant option(s) with an [x] -->

- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)
- [ ] ✨ New feature (non-breaking change that adds functionality)
- [ ] 💥 Breaking change (fix or feature that causes existing functionality to change)
- [ ] 📚 Documentation update
- [ ] 🎨 Code style/formatting change
- [ ] ♻️  Refactoring (no functional changes)
- [ ] ⚡ Performance improvement
- [ ] ✅ Test addition or improvement
- [ ] 🔧 Build/CI configuration change
- [ ] 🏷️  Type definition update

## Related Issues

<!-- Link any related issues using "Fixes #123" or "Closes #456" -->

- Fixes #
- Related to #

## Changes Made

<!-- Describe the changes you made in detail -->

### Core Changes

- [ ] Modified scanner logic
- [ ] Updated parser implementation
- [ ] Changed composer behavior
- [ ] Modified emitter output
- [ ] Updated error handling
- [ ] Changed public API

### Specific Changes

-
-
-

## YAML Features Affected

<!-- Mark any YAML features that are affected by your changes -->

- [ ] Basic parsing (scalars, sequences, mappings)
- [ ] Advanced parsing (anchors, aliases, merge keys)
- [ ] Multi-line strings (literal, folded)
- [ ] Type tags (!!str, !!int, etc.)
- [ ] Complex keys
- [ ] Serialization/dumping
- [ ] Round-trip preservation
- [ ] Error handling
- [ ] Performance
- [ ] Multi-document support
- [ ] Style preservation
- [ ] Configuration/loaders

## Testing

<!-- Describe your testing approach -->

### Test Coverage

- [ ] Added new unit tests
- [ ] Added new integration tests
- [ ] Updated existing tests
- [ ] Added performance benchmarks
- [ ] Added fuzz testing cases
- [ ] Manual testing completed

### Test Commands Run

```bash
# List the commands you ran to test your changes
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```

### Test Results

<!-- Paste relevant test output or describe test results -->

```
# Paste test output here if relevant
```

## Performance Impact

<!-- Describe any performance implications -->

- [ ] No performance impact expected
- [ ] Performance improvement (describe below)
- [ ] Performance degradation (describe and justify below)
- [ ] Unknown performance impact

### Performance Details

<!-- If there's a performance impact, describe it here -->

## Breaking Changes

<!-- If this is a breaking change, describe the impact -->

- [ ] No breaking changes
- [ ] Breaking changes (describe below)

### Breaking Change Details

<!-- Describe any breaking changes and migration path -->

## Security Considerations

<!-- Describe any security implications -->

- [ ] No security implications
- [ ] Security improvement (describe below)
- [ ] Potential security impact (describe below)

### Security Details

<!-- Describe security considerations if applicable -->

## Documentation

<!-- How did you update documentation? -->

- [ ] No documentation changes needed
- [ ] Updated API documentation
- [ ] Updated README
- [ ] Updated examples
- [ ] Updated CHANGELOG
- [ ] Added inline code comments

## Checklist

<!-- Ensure all items are complete before submitting -->

### Code Quality

- [ ] Code follows the project's coding standards
- [ ] Code is properly formatted (`cargo fmt`)
- [ ] Code passes linting (`cargo clippy -- -D warnings`)
- [ ] No compiler warnings introduced
- [ ] All tests pass (`cargo test`)

### Documentation

- [ ] Public APIs are documented
- [ ] Examples are provided for new features
- [ ] CHANGELOG.md updated (for significant changes)
- [ ] Breaking changes documented

### Testing

- [ ] Existing tests still pass
- [ ] New tests added for new functionality
- [ ] Edge cases tested
- [ ] Error conditions tested
- [ ] Performance regression tested (if applicable)

### Review Readiness

- [ ] PR is based on the latest main branch
- [ ] PR has a clear, descriptive title
- [ ] PR description explains the what and why
- [ ] Commits are logically organized
- [ ] Commit messages follow conventional format

## Additional Context

<!-- Add any additional context, screenshots, or information -->

### Screenshots

<!-- If applicable, add screenshots to help explain your changes -->

### Additional Notes

<!-- Any additional information for reviewers -->

---

## For Reviewers

### Review Focus Areas

<!-- Suggest areas for reviewers to focus on -->

- [ ] Correctness of YAML parsing/generation
- [ ] YAML 1.2 specification compliance
- [ ] Error handling robustness
- [ ] Performance implications
- [ ] API design and usability
- [ ] Test coverage adequacy
- [ ] Documentation clarity
- [ ] Security considerations

### Testing Suggestions

<!-- Suggest specific testing approaches for reviewers -->

- Test with malformed YAML input
- Test with edge cases and boundary conditions
- Verify round-trip preservation
- Check error message quality
- Validate performance with large inputs

### Questions for Reviewers

<!-- Any specific questions you have for reviewers -->

-
-

---

**Thank you for contributing to rust-yaml! 🎉**
