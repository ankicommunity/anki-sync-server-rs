# How to contribute to this project

First of all thanks for the interest and your willingness to give your precious time to this project!

If you want to ensure your contribution acceptance please open an issue before contributing.

We use the classical forge fork and merge pipeline.

## Setup the development environment

Prerequisite:
- rust dev toolchain (see [rustup](https://rustup.rs/) if needed) including `cargo`, `cargo check`,`cargo clippy`,
- [pre-commit](https://pre-commit.com/) (install using `pip install --user pre-commit`  if needed)

Steps:
1. Fork this this repository, then clone the fork using git and enter it
2. Install precommit hooks `pre-commit install --install-hooks`
3. Checkout a branch for your contribution `git checkout -b my-new-feature`
4. Push your new feature
5. Open an MR at [ankicommunity/anki-sync-server-rs](https://github.com/ankicommunity/anki-sync-server-rs)

## During development

Use  `scripts/build_all` to check if building works with each feature.
