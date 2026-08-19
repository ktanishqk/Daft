# Resources

- https://docs.daft.ai for the user-facing API docs
- CONTRIBUTING.md for detailed development process
- https://github.com/Eventual-Inc/Daft for issues, discussions, and PRs

# Dev Workflow

1. [Once] Set up Python environment and install dependencies: `make .venv`
2. [Optional] Activate .venv: `source .venv/bin/activate`. Not necessary with Makefile commands.
3. If Rust code is modified, rebuild: `make build`
4. Run tests. See [Testing Details](#testing-details).

# Testing Details

- `make test` runs tests in `tests/` directory. Uses `pytest` under the hood.
  - Must set `DAFT_RUNNER` environment variable to `ray` or `native` to run the tests with the corresponding runner.
    - Start with `DAFT_RUNNER=native` unless testing Ray or distributed code.
  - `make test EXTRA_ARGS="..."` passes additional arguments to `pytest`.
    - `make test EXTRA_ARGS="-v tests/dataframe/test_select.py"` runs the test in the given file.
    - `make test EXTRA_ARGS="-v tests/dataframe/test_select.py::test_select_dataframe"` runs the given test method.
  - Default `integration`, `benchmark`, and `hypothesis` tests are disabled. Best to run on CI.
- `make doctests` runs doctests in `daft/` directory. Tests docstrings in Daft APIs.

# PR Conventions

- Titles: Conventional Commits format; enforced by `.github/workflows/pr-labeller.yml`.
- Descriptions: follow `.github/pull_request_template.md`.

## Cursor Cloud specific instructions

Daft is a single product: a Python library (`daft`) backed by a Rust workspace (compiled into `daft/daft.abi3.so`). There is no server/frontend to run — "running the app" means importing `daft` and executing dataframe/SQL operations.

- The startup update script installs `uv` and runs `make .venv` (deps only). The compiled Rust extension is carried in the VM snapshot, so a booted agent can normally `import daft` immediately without rebuilding.
- After changing Rust code under `src/`, you must run `make build` to recompile (first clean build is ~6 min; incremental is seconds). Python-only changes need no rebuild since `daft` is installed editable.
- `make .venv` is a Make *file target*: it is a no-op whenever the `.venv/` directory already exists. To pick up dependency changes from `uv.lock`, run `rm -rf .venv && make .venv` (or `uv sync --no-install-project --all-extras --all-groups`). Note that a bare `uv sync` uninstalls the maturin editable `daft` dist-info; re-run `make build` afterward to restore a robust (cwd-independent) editable install.
- Always set `DAFT_RUNNER` when running code/tests: use `DAFT_RUNNER=native` by default; `DAFT_RUNNER=ray` for distributed paths.
- Lint uses pinned pre-commit hook versions, not the `ruff` in `.venv` (which is newer and stricter). Run lint via `make lint` / `pre-commit run ruff-check --all-files`; do not judge lint by calling `.venv/bin/ruff` directly.
