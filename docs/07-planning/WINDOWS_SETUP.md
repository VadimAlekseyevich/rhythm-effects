# Windows Development Setup

> **Status: Accepted bootstrap for the current M1 scaffold**
>
> Target: Windows 10/11 x86_64, repository at C:\Projects\rhythm-effects.

## 1. Install prerequisites

Run **Command Prompt as Administrator**:

~~~bat
winget install --id Git.Git -e --source winget --accept-source-agreements --accept-package-agreements

winget install --id Microsoft.VisualStudio.BuildTools -e --source winget --accept-source-agreements --accept-package-agreements --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

winget install --id Rustlang.Rustup -e --source winget --accept-source-agreements --accept-package-agreements
~~~

Close Command Prompt after installers finish and open a new ordinary Command Prompt so PATH changes are visible.

## 2. Install the repository toolchain explicitly

~~~bat
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
rustup default 1.98.1
~~~

The repository also contains rust-toolchain.toml, so commands run inside the repository select 1.98.1.

## 3. Verify tools

~~~bat
git --version
rustup --version
rustc --version
cargo --version
~~~

## 4. Clone into C:\Projects

~~~bat
if not exist C:\Projects mkdir C:\Projects
cd /d C:\Projects

git clone https://github.com/VadimAlekseyevich/rhythm-effects.git
cd /d C:\Projects\rhythm-effects
~~~

If the repository is already cloned:

~~~bat
cd /d C:\Projects\rhythm-effects
git pull --ff-only
~~~

## 5. Verify repository state

~~~bat
git status
rustup show active-toolchain
~~~

Expected active toolchain contains 1.98.1.

## 6. Download Rust dependencies and create Cargo.lock

~~~bat
cargo fetch
~~~

Cargo creates/updates Cargo.lock for the workspace.

Because Rhythm Effects is an application workspace, Cargo.lock should be committed once generated and validated.

## 7. Run the quality checks

~~~bat
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
~~~

All four commands must pass before continuing M1 implementation.

## 8. Run Rhythm Effects

~~~bat
cargo run -p rhythm_app
~~~

Expected current scaffold behavior:

- a native window titled "Rhythm Effects" opens;
- minimum size is 1280×720;
- default size is 1440×900;
- console shows tracing startup/window messages;
- closing the window exits cleanly.

The current scaffold intentionally stops before wgpu/egui initialization. The next tasks in AI_EXECUTION_TASKS.md add those pieces one reviewable change at a time.

## 9. Commit the generated lockfile

After the first successful local build:

~~~bat
git status
git add Cargo.lock
git commit -m "chore: lock initial Rust dependencies"
git push
~~~

Git for Windows includes Git Credential Manager; if GitHub authentication is needed for push, complete the browser sign-in prompt.

## 10. Optional developer editor

VS Code is not required to build the project. If desired:

~~~bat
winget install --id Microsoft.VisualStudioCode -e --source winget
~~~

Then install the rust-analyzer extension from VS Code.

## 11. Not needed yet

Do not install solely for the current scaffold:

- Node.js;
- Python;
- separate CMake;
- FFmpeg;
- audio SDKs.

Add tools only when the corresponding accepted implementation task requires them.
