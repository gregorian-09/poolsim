# Packaging

Poolsim includes packaging assets for users who do not want to build from source manually.

## Homebrew Tap Formula

The Homebrew formula template is checked in at `packaging/homebrew/poolsim.rb`.

It builds and installs:

- `poolsim` from `crates/poolsim-cli`
- `poolsim-web` from `crates/poolsim-web`

### Release SHA Step

Before publishing the formula to a tap, replace:

```ruby
sha256 "REPLACE_WITH_RELEASE_TARBALL_SHA256"
```

with the SHA-256 of the GitHub release tarball for the matching tag:

```bash
curl -L https://github.com/gregorian-09/poolsim/archive/refs/tags/v0.2.1.tar.gz -o poolsim-v0.2.1.tar.gz
sha256sum poolsim-v0.2.1.tar.gz
```

Homebrew's formula cookbook documents formula creation, installation, and debugging through `brew create`, `brew install --debug --verbose`, and tap repositories.

### Install From A Tap

After the tap repository publishes the formula:

```bash
brew tap gregorian-09/poolsim
brew install poolsim
```

Upgrade later with:

```bash
brew update
brew upgrade poolsim
```

## Validate Formula Metadata

```bash
python3 -m unittest tests/packaging/test_homebrew_formula.py
```

## Source

- Homebrew Formula Cookbook: https://docs.brew.sh/Formula-Cookbook
