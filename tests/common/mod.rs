//! Shared test helpers for integration and E2E tests.
//!
//! # Isolation requirement
//!
//! Every integration test that touches git, the filesystem, or home-directory
//! behaviour MUST use [`GitEnv`] rather than creating temp dirs + host git
//! commands directly.  `GitEnv` backs each test with a Docker container
//! (via testcontainers) that provides a fully isolated git environment:
//! clean `gitconfig`, no host hooks, no leaked `GIT_*` env vars, and a pinned
//! git version.
//!
//! # Usage
//!
//! ```rust,ignore
//! mod common;
//!
//! #[tokio::test]
//! async fn my_test() {
//!     let env = common::GitEnv::new().await;
//!
//!     // git setup runs in the container (isolated)
//!     env.git(&["checkout", "-b", "feature"]).await;
//!
//!     // `bs` runs on the host against the shared bind-mount
//!     let slot = env.run_get();
//!     assert!(slot.exists());
//! } // container + temp dirs cleaned up on drop
//! ```
//!
//! # Extending
//!
//! Add new helpers in this file for each service type your tests need.
//! Keep helpers focused: one function/struct per concern.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{ExecCommand, Mount, WaitFor},
    runners::AsyncRunner,
};

// ── hello-world canary ────────────────────────────────────────────────────────

/// Start a lightweight `hello-world` container and return the handle.
///
/// This is the canary test for the Docker/testcontainers infrastructure.
/// If it passes, Docker is reachable and the async runner works correctly.
///
/// # Panics
///
/// Panics if Docker is unavailable or the image cannot be pulled.
pub async fn start_generic_container() -> ContainerAsync<GenericImage> {
    GenericImage::new("hello-world", "latest")
        .with_wait_for(WaitFor::message_on_stdout("Hello from Docker!"))
        .start()
        .await
        .expect("failed to start hello-world container — is Docker running?")
}

// ── GitEnv ────────────────────────────────────────────────────────────────────

/// Pinned `alpine/git` image used for all git-backed integration tests.
/// `latest` is used here for convenience; a CI pipeline should pin this to
/// a specific digest via `docker pull alpine/git:latest && docker inspect …`
/// and substitute the sha256 reference via `with_name`.
const GIT_IMAGE: &str = "alpine/git";
const GIT_TAG: &str = "latest";

/// A fully isolated git test environment backed by a Docker container.
///
/// - An `alpine/git` container is started and kept alive for the lifetime of
///   the test.  The container's entrypoint is overridden to `sh -c "tail -f
///   /dev/null"` so that `exec` commands can be issued at any point.
/// - The git repository lives in a host `TempDir` that is bind-mounted into
///   the container at `/workspace`, so both the container's `git` binary and
///   the host `bs` binary operate on the same files.
/// - `BONSAI_ROOT` is a separate host `TempDir`; `bs get` writes its managed
///   worktree pool there instead of `~/.bonsai`.
///
/// All git *setup* and *mutation* commands (init, config, add, commit, …)
/// MUST be run via [`GitEnv::git`] (container exec) to stay fully isolated
/// from the host machine's git configuration and environment variables.
pub struct GitEnv {
    /// Keeps the container alive until the test ends.
    _container: ContainerAsync<GenericImage>,
    _repo: TempDir,
    _bonsai: TempDir,
    /// Canonicalised host path of the git repository.
    /// Matches the container's `/workspace`.
    pub repo_path: PathBuf,
    /// Canonicalised host path used as `BONSAI_ROOT`.
    pub bonsai_path: PathBuf,
}

impl GitEnv {
    /// Start the git container and initialise a fresh repository inside it.
    ///
    /// After `new()` returns the repository has one commit (`init`) and the
    /// container is ready to accept further `exec` commands.
    ///
    /// # Panics
    ///
    /// Panics if Docker is unavailable, the image cannot be pulled, or the
    /// initial `git init` / commit fails.
    pub async fn new() -> Self {
        // Canonicalise so Docker Desktop's bind-mount path resolution works
        // on macOS (/var/folders symlink -> /private/var/folders).
        let repo_tmp = TempDir::new().expect("temp repo dir");
        let bonsai_tmp = TempDir::new().expect("temp bonsai dir");
        let repo_path = repo_tmp.path().canonicalize().expect("canonicalize repo");
        let bonsai_path = bonsai_tmp
            .path()
            .canonicalize()
            .expect("canonicalize bonsai");

        let container = GenericImage::new(GIT_IMAGE, GIT_TAG)
            .with_entrypoint("sh")
            // with_wait_for must be called on GenericImage before any ImageExt
            // method (which converts it to ContainerRequest).
            .with_wait_for(WaitFor::Nothing)
            // ImageExt methods follow.
            .with_cmd(["-c", "tail -f /dev/null"])
            .with_mount(Mount::bind_mount(
                repo_path.to_str().expect("repo path is valid utf-8"),
                "/workspace",
            ))
            .start()
            .await
            .expect(
                "failed to start git container — is Docker running? \
                 Does Docker Desktop have file sharing enabled for this path?",
            );

        let env = GitEnv {
            _container: container,
            _repo: repo_tmp,
            _bonsai: bonsai_tmp,
            repo_path,
            bonsai_path,
        };

        // Initialise the repository inside the container (clean environment:
        // no host gitconfig, no host hooks, no GIT_* env vars).
        env.git(&["init"]).await;
        env.git(&["config", "user.email", "test@example.com"]).await;
        env.git(&["config", "user.name", "Test"]).await;
        env.git(&["config", "commit.gpgsign", "false"]).await;

        // Write the initial file from the host (simpler than exec + redirection)
        // then commit via the container.
        std::fs::write(env.repo_path.join("README.md"), "# test").expect("write README.md");
        env.git(&["add", "."]).await;
        env.git(&["commit", "-m", "init"]).await;

        env
    }

    /// Run a `git` command inside the container at `/workspace`.
    ///
    /// Equivalent to: `git -C /workspace <args…>`
    ///
    /// # Panics
    ///
    /// Panics if the command exits with a non-zero status.
    pub async fn git(&self, args: &[&str]) {
        use tokio::io::AsyncReadExt as _;

        let cmd: Vec<String> = ["git", "-C", "/workspace"]
            .iter()
            .chain(args.iter())
            .map(|s| s.to_string())
            .collect();

        // Use `exit_code(0)` so testcontainers blocks until the command
        // finishes before returning the ExecResult (the default
        // `CmdWaitFor::Nothing` returns immediately without waiting).
        let exec_cmd = ExecCommand::new(cmd.clone())
            .with_cmd_ready_condition(testcontainers::core::CmdWaitFor::exit_code(0));

        let mut result = self
            ._container
            .exec(exec_cmd)
            .await
            .unwrap_or_else(|e| panic!("container exec failed for {cmd:?}: {e}"));

        // Read stderr so we can include it in the panic message on failure.
        let mut stderr_bytes = Vec::new();
        result.stderr().read_to_end(&mut stderr_bytes).await.ok();

        let exit = result
            .exit_code()
            .await
            .unwrap_or_else(|e| panic!("get exit code for {cmd:?}: {e}"))
            .expect("exit code should be set after CmdWaitFor::exit_code");

        assert_eq!(
            exit,
            0,
            "git {args:?} exited with status {exit}:\n{}",
            String::from_utf8_lossy(&stderr_bytes)
        );
    }

    /// Returns a `Command` builder for the `bs` binary with:
    /// - `current_dir` = `self.repo_path`
    /// - `BONSAI_ROOT` = `self.bonsai_path`
    pub fn bs(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_bs"));
        cmd.current_dir(&self.repo_path)
            .env("BONSAI_ROOT", &self.bonsai_path);
        cmd
    }

    /// Returns a `Command` builder for `bs` with `current_dir` overridden to
    /// `dir` (useful for testing behaviour from inside a linked worktree).
    pub fn bs_from(&self, dir: &Path) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_bs"));
        cmd.current_dir(dir).env("BONSAI_ROOT", &self.bonsai_path);
        cmd
    }

    /// Run `bs get`, assert success, and return the slot path parsed from
    /// stdout.
    pub fn run_get(&self) -> PathBuf {
        let out = self.bs().arg("get").output().expect("spawn bs get");
        assert!(
            out.status.success(),
            "bs get failed\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        path_from_output(&out.stdout)
    }

    /// HEAD SHA of the test repository.
    pub fn head_sha(&self) -> String {
        worktree_head(&self.repo_path)
    }

    /// Write a file and create a new commit (via container exec).
    pub async fn make_commit(&self, filename: &str, content: &str) {
        std::fs::write(self.repo_path.join(filename), content)
            .unwrap_or_else(|e| panic!("write {filename}: {e}"));
        self.git(&["add", "."]).await;
        self.git(&["commit", "-m", &format!("add {filename}")])
            .await;
    }

    /// The single repo-slug directory created under `bonsai_path` after the
    /// first `bs get`.  Panics if there isn't exactly one.
    pub fn slug_dir(&self) -> PathBuf {
        let entries: Vec<_> = std::fs::read_dir(&self.bonsai_path)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().canonicalize().unwrap_or_else(|_| e.path()))
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "expected exactly one slug dir under bonsai_path, got: {entries:?}"
        );
        entries.into_iter().next().unwrap()
    }

    /// All slot directories under the slug directory (sorted).
    pub fn slots(&self) -> Vec<PathBuf> {
        let mut v: Vec<_> = std::fs::read_dir(self.slug_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        v.sort();
        v
    }
}

// ── Shared utilities ──────────────────────────────────────────────────────────

/// Parse the worktree path from `bs get` stdout.
///
/// stdout format: `🌳 /absolute/path/to/slot\n`
pub fn path_from_output(raw: &[u8]) -> PathBuf {
    let text = String::from_utf8_lossy(raw);
    PathBuf::from(text.trim().trim_start_matches('🌳').trim())
}

/// HEAD SHA of the worktree at `path`.
///
/// Uses the host `git` binary with all hook-injected `GIT_*` env vars
/// cleared, so this is safe to call from within a pre-commit hook.
pub fn worktree_head(path: &Path) -> String {
    let out = host_git(path, &["rev-parse", "HEAD"]);
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Run a `git` command on the HOST in `dir` with hook env vars cleared.
///
/// Suitable for read-only assertions (e.g. `rev-parse HEAD`) and for
/// mutations that must run on the host (e.g. `git worktree lock`).
pub fn host_git(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new("git");
    for var in [
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_WORK_TREE",
        "GIT_PREFIX",
        "GIT_COMMON_DIR",
    ] {
        cmd.env_remove(var);
    }
    cmd.args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("host git {args:?} failed: {e}"))
}
