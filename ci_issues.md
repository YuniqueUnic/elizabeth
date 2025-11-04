## `release-plz-pr` creates PR without version bump for packages not published to crates.io

### Summary

I'm trying to set up a CI/CD workflow for a Rust workspace that publishes GitHub
Releases (not to crates.io). The `release-plz-pr` workflow is triggered
correctly on pushes to `main`, but the Pull Request it creates does **not** bump
the version in `Cargo.toml`. This causes the subsequent `release-plz-release`
workflow to do nothing, as it sees no version change, effectively halting the
entire release process. The key log message seems to be
`WARN Package \`elizabeth-board@_._.*\` not
found`, which suggests`release-plz`is looking for the package on`crates.io` and,
failing to find it, cannot determine the next version.

#### Environment

- **release-plz version:** `0.3.148`
- **OS:** `ubuntu-latest` (GitHub Actions)
- **Rust version:** `stable`

#### Expected Behavior

My desired CI/CD workflow is as follows:

1. A user merges a PR into the `main` branch.
2. This triggers the `release-plz-pr` job.
3. The `release-plz-pr` job automatically creates a new PR, updating the
   `CHANGELOG.md` and bumping the version in `Cargo.toml`.
4. Once this release PR is merged, it triggers the `release-plz-release` job.
5. The `release-plz-release` job detects the version update, creates a GitHub
   Release/Tag, and triggers a downstream CD workflow.

#### Actual Behavior

The workflow is stuck at step 3.

1. The `release-plz-pr` job is triggered and creates a PR, but the version in
   `Cargo.toml` is **not** updated.
2. The `release-plz-release` job is triggered (by merging a different PR or
   manually), but it does nothing because the version hasn't changed. **Key Log
   from `release-plz-pr` job:**

```
2025-11-03T00:42:42.375834Z  WARN Package `elizabeth-board@*.*.*` not found
2025-11-03T00:42:42.413637Z  INFO determining next version for elizabeth-board 0.3.0
2025-11-03T00:42:52.721714Z  INFO elizabeth-board: next version is 0.3.0
```

The "next version" is the same as the current version, so no change is made in
the PR. **Key Log from `release-plz-release` job:**

```
2025-11-04T14:29:23.711978Z  INFO elizabeth-board 0.3.0: Already published - Tag v0.3.0 already exists
release_output: {"releases":[]}
```

This confirms that `release-plz` sees no new version to release.

#### Hypothesis

The issue likely stems from the fact that my packages are **not published to
crates.io**. The `release-plz-pr` command seems to rely on querying the registry
to find the last published version to calculate the next one. Since it can't
find `elizabeth-board` on `crates.io`, it defaults to the current workspace
version (`0.3.0`), resulting in no version bump. My goal is to manage versions
and releases purely through Git tags and GitHub Releases, without involving
`crates.io`.

### My configurations

#### release-plz-pr.yaml

```yaml
name: release-plz-pr

on:
  push:
    branches:
      - main
    # 忽略只修改了这些文件的 push（通常是 release-plz 的更改）
    # ignore these files changes which usually come from release-plz-pr
    # avoid recursive release-plz-pr
    paths-ignore:
      - "CHANGELOG.md"
      - "**/Cargo.toml"
      - "Cargo.lock"
  schedule:
    - cron: "0 0 * * MON"

jobs:
  release-plz-pr:
    name: Create Release PR
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
      contents: write
    concurrency:
      group: release-plz-${{ github.ref }}
      cancel-in-progress: false
    steps:
      - name: Checkout repository
        uses: actions/checkout@v5
        with:
          fetch-depth: 0
          persist-credentials: false

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Run release-plz PR
        uses: release-plz/action@v0.5
        with:
          command: release-pr
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

```ci
Run release-plz/action@v0.5
Run taiki-e/install-action@v2
Run set -eu
Run bash --noprofile --norc "${GITHUB_ACTION_PATH:?}/main.sh"
info: host platform: x86_64_linux
info: installing cargo-semver-checks@0.43
info: downloading https://github.com/obi1kenobi/cargo-semver-checks/releases/download/v0.43.0/cargo-semver-checks-x86_64-unknown-linux-musl.tar.gz
info: verifying sha256 checksum for cargo-semver-checks-x86_64-unknown-linux-musl.tar.gz
info: cargo-semver-checks installed at /home/runner/.cargo/bin/cargo-semver-checks
+ cargo-semver-checks semver-checks --version
cargo-semver-checks 0.43.0

info: installing release-plz@0.3.148
info: downloading https://github.com/release-plz/release-plz/releases/download/release-plz-v0.3.148/release-plz-x86_64-unknown-linux-musl.tar.gz
info: verifying sha256 checksum for release-plz-x86_64-unknown-linux-musl.tar.gz
info: release-plz installed at /home/runner/.cargo/bin/release-plz
+ release-plz --version
release-plz 0.3.148

Run release-plz/git-config@59144859caf016f8b817a2ac9b051578729173c4
Run if [ -z "${GITHUB_TOKEN+x}" ]; then
Run if [[ -n "" ]]
Using forge 'github'
-- Running release-plz release-pr --
2025-11-03T00:42:42.202087Z  INFO using release-plz config file .release-plz.toml
2025-11-03T00:42:42.238598Z  INFO downloading packages from cargo registry crates.io
    Updating crates.io index
2025-11-03T00:42:42.375834Z  WARN Package `elizabeth-board@*.*.*` not found
2025-11-03T00:42:42.413637Z  INFO determining next version for elizabeth-board 0.3.0
2025-11-03T00:42:52.721714Z  INFO elizabeth-board: next version is 0.3.0
2025-11-03T00:42:56.506666Z  INFO opened pr: https://github.com/YuniqueUnic/elizabeth/pull/35
release_pr_output: {"prs":[{"base_branch":"main","head_branch":"release-plz-2025-11-03T00-42-54Z","html_url":"https://github.com/YuniqueUnic/elizabeth/pull/35","number":35,"releases":[{"package_name":"elizabeth-board","version":"0.3.0"}]}]}
```

#### release-plz-release.yaml

```yaml
name: release-plz-release

on:
  push:
    branches:
      - main
    # 只在这些文件被修改时触发（通常是 release-plz PR 被合并）
    # Only trigger on these files changes which usually come from release-plz-pr
    paths:
      - "CHANGELOG.md"
      - "**/Cargo.toml"
      - "Cargo.lock"
  workflow_dispatch:

permissions: {}

jobs:
  release-plz-release:
    name: Publish Release
    runs-on: ubuntu-latest
    if: github.repository_owner == 'yuniqueunic'
    permissions:
      pull-requests: write
      contents: write
      id-token: write
    steps:
      - name: Checkout repository
        uses: actions/checkout@v5
        with:
          fetch-depth: 0
          persist-credentials: false

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      # 生成 GitHub App token，确保创建的 release 能触发 CD workflow
      # Generate GitHub App token to ensure the created release can trigger CD workflow
      - name: Generate GitHub token
        uses: actions/create-github-app-token@v2
        id: generate-token
        with:
          app-id: ${{ secrets.APP_ID }}
          private-key: ${{ secrets.APP_PRIVATE_KEY }}
          permission-contents: write
          permission-pull-requests: read

      - name: Run release-plz release
        uses: release-plz/action@v0.5
        with:
          command: release
        env:
          GITHUB_TOKEN: ${{ steps.generate-token.outputs.token }}
```

```ci
Run release-plz/action@v0.5
Run taiki-e/install-action@v2
Run set -eu
Run bash --noprofile --norc "${GITHUB_ACTION_PATH:?}/main.sh"
info: host platform: x86_64_linux
info: installing cargo-semver-checks@0.43
info: downloading https://github.com/obi1kenobi/cargo-semver-checks/releases/download/v0.43.0/cargo-semver-checks-x86_64-unknown-linux-musl.tar.gz
info: verifying sha256 checksum for cargo-semver-checks-x86_64-unknown-linux-musl.tar.gz
info: cargo-semver-checks installed at /home/runner/.cargo/bin/cargo-semver-checks
+ cargo-semver-checks semver-checks --version
cargo-semver-checks 0.43.0

info: installing release-plz@0.3.148
info: downloading https://github.com/release-plz/release-plz/releases/download/release-plz-v0.3.148/release-plz-x86_64-unknown-linux-musl.tar.gz
info: verifying sha256 checksum for release-plz-x86_64-unknown-linux-musl.tar.gz
info: release-plz installed at /home/runner/.cargo/bin/release-plz
+ release-plz --version
release-plz 0.3.148

Run release-plz/git-config@59144859caf016f8b817a2ac9b051578729173c4
Run if [ -z "${GITHUB_TOKEN+x}" ]; then
Run if [[ -n "" ]]
Using forge 'github'
-- Running release-plz release --
2025-11-04T14:29:22.602849Z  INFO using release-plz config file .release-plz.toml
2025-11-04T14:29:23.711978Z  INFO elizabeth-board 0.3.0: Already published - Tag v0.3.0 already exists
release_output: {"releases":[]}
```

#### .release-plz.toml

```toml
[workspace]
changelog_path = "CHANGELOG.md"
changelog_update = true
dependencies_update = true  # run `cargo update` in the release PR
git_release_enable = false  # workspace release disabled
git_tag_enable = false      # workspace tag disabled
publish = false
semver_check = true
pr_branch_prefix = "release-plz-"
pr_labels = ["release"]
publish_timeout = "10m"

# only configure the board (which is the main package)
[[package]]
name = "elizabeth-board"
changelog_path = "CHANGELOG.md"
publish = false
semver_check = true
git_release_enable = true   # only the board creates a GitHub release
git_tag_enable = true       # only the board creates a GitHub tag
git_release_name = "elizabeth-board-v{{ version }}"
git_tag_name = "v{{ version }}"

[[package]]
name = "elizabeth-configrs"
release = false

[[package]]
name = "elizabeth-logrs"
release = false
```

#### my project structure

```bash
crates/:
    |--board/
        [package]
        name = "elizabeth-board"
        version.workspace = true
    |--configrs/
        [package]
        name = "elizabeth-configrs"
        version.workspace = true
    |--logrs/
        [package]
        name = "elizabeth-logrs"
        version.workspace = true
Cargo.toml:
    [workspace.package]
    version = "0.3.0"
```

### Question

Is this the expected behavior for packages not published to a registry?

Is there a recommended configuration or a different approach to achieve a
Git-based release workflow without publishing to `crates.io`?

Perhaps a configuration option to tell `release-plz` to use the latest Git tag
as the version baseline instead of querying the registry?

Thank you for your help and for this great tool!
