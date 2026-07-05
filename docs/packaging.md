# Packaging

Poolsim includes packaging assets for users who do not want to build from source manually.

## Homebrew Tap Formula

The Homebrew formula is checked in at `packaging/homebrew/poolsim.rb`.

It builds and installs:

- `poolsim` from `crates/poolsim-cli`
- `poolsim-web` from `crates/poolsim-web`

The formula must point to an immutable GitHub release tarball and matching SHA-256. During release preparation, keep the formula on the last published tag until the new tag exists. After creating the `v0.3.0` tag, compute the checksum and update `packaging/homebrew/poolsim.rb` before publishing the tap formula:

```bash
curl -L https://github.com/gregorian-09/poolsim/archive/refs/tags/v0.3.0.tar.gz -o poolsim-v0.3.0.tar.gz
sha256sum poolsim-v0.3.0.tar.gz
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
