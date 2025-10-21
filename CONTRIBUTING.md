# Contributing to Ghost SSH Manager

Thank you for your interest in contributing to Ghost SSH Manager! We welcome contributions from the community.

## 🚀 Getting Started

### Prerequisites
- Rust 1.70.0 or higher
- Git
- Terminal with true color support

### Development Setup

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/Smoke516/ghost.git
   cd ghost
   ```

2. **Build the project**
   ```bash
   cargo build
   ```

3. **Run the application**
   ```bash
   cargo run
   ```

4. **Run tests**
   ```bash
   cargo test
   cargo clippy
   cargo fmt
   ```

## 📋 How to Contribute

### Reporting Issues
- Use the [GitHub Issues](https://github.com/Smoke516/ghost/issues) page
- Search existing issues before creating a new one
- Provide detailed information including:
  - Operating system and version
  - Terminal emulator being used
  - Steps to reproduce the issue
  - Expected vs actual behavior
  - Screenshots if applicable

### Suggesting Features
- Open an issue with the "enhancement" label
- Describe the feature and its use case
- Explain how it fits with the project's goals
- Consider backward compatibility

### Submitting Changes

1. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Follow the coding standards below
   - Add tests for new functionality
   - Update documentation as needed

3. **Test your changes**
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

4. **Commit your changes**
   - Use clear, descriptive commit messages
   - Follow conventional commits format:
     ```
     feat: add session management feature
     fix: resolve terminal detection on Windows
     docs: update installation instructions
     ```

5. **Push and create a Pull Request**
   ```bash
   git push origin feature/your-feature-name
   ```

## 🎯 Coding Standards

### Code Style
- Use `cargo fmt` for consistent formatting
- Follow Rust naming conventions
- Add documentation comments for public APIs
- Keep functions focused and reasonably sized

### Testing
- Write unit tests for new functionality
- Test cross-platform compatibility when possible
- Include integration tests for major features
- Aim for good test coverage

### Documentation
- Update README.md for user-facing changes
- Add inline documentation for complex code
- Update help text and UI tooltips
- Include examples in documentation

## 🏗️ Project Structure

```
src/
├── main.rs          # Application entry point
├── app.rs           # Main application logic and event handling  
├── colors.rs        # Tokyo Night color palette
├── models.rs        # Data structures and state management
└── ui/
    └── mod.rs       # Terminal UI components and rendering
```

## 🎨 UI/UX Guidelines

- Maintain consistency with the Tokyo Night theme
- Ensure responsive design across terminal sizes
- Provide clear visual feedback for user actions
- Include helpful tooltips and contextual help
- Test with different terminal emulators

## 🔒 Security Considerations

- Never log or store sensitive information (passwords, keys)
- Use secure defaults for SSH connections
- Validate all user inputs
- Follow secure coding practices
- Report security issues privately

## 📱 Cross-Platform Development

### Testing Platforms
- **Linux**: Test on major distributions (Ubuntu, Fedora, Arch)
- **macOS**: Test on recent versions (10.15+)
- **Windows**: Test on Windows 10/11

### Platform-Specific Code
- Use conditional compilation when needed: `#[cfg(target_os = "...")]`
- Test terminal detection across platforms
- Handle path differences appropriately
- Consider platform-specific terminal behaviors

## 🚦 Pull Request Process

1. **Pre-submission checklist**
   - [ ] Code compiles without warnings
   - [ ] Tests pass locally
   - [ ] Documentation updated
   - [ ] CHANGELOG.md updated (for significant changes)

2. **PR Review Process**
   - Maintainers will review within 1-2 weeks
   - Address feedback promptly
   - Keep discussions respectful and constructive
   - Be patient during the review process

3. **Merge Requirements**
   - At least one maintainer approval
   - All CI checks passing
   - No merge conflicts
   - Up-to-date with main branch

## 🎯 Areas for Contribution

### High Priority
- Cross-platform testing and bug fixes
- Performance optimizations
- Security enhancements
- Terminal compatibility improvements

### Medium Priority  
- Additional themes and customization options
- Enhanced analytics and reporting
- SSH key management features
- Connection history improvements

### Good First Issues
- Documentation improvements
- Minor UI/UX enhancements
- Error message improvements
- Help text updates

## 📞 Getting Help

- **Discussions**: Use [GitHub Discussions](https://github.com/Smoke516/ghost/discussions)
- **Chat**: Join our community discussions
- **Issues**: Create an issue for bugs or feature requests

## 🎉 Recognition

Contributors will be:
- Listed in the project's contributors
- Mentioned in release notes for significant contributions
- Credited in documentation where appropriate

## 📜 Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help create a welcoming environment for all contributors
- Follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct)

---

Thank you for contributing to Ghost SSH Manager! Your efforts help make SSH management better for everyone. 👻