//! Branch-state check: does a clean slot's branch carry unmerged work?
//!
//! Uses only local git state (refs, config, objects): no network, no fetch.
//! Whenever merged-ness cannot be proven the answer errs toward "unmerged"
//! (the slot is `in use`).

use std::path::Path;

use super::git_cmd;

/// The ref a slot's work is compared against (the "default ref" D).
///
/// `None` means no default ref could be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultRef(pub Option<String>);

impl DefaultRef {
    fn name(&self) -> &str {
        self.0.as_deref().unwrap_or("the default branch")
    }
}

/// A default ref resolved at most once, on first use, and shareable across
/// per-slot threads (locked/dirty slots never pay for the resolution).
#[derive(Debug)]
pub struct LazyDefaultRef {
    dir: std::path::PathBuf,
    cell: std::sync::OnceLock<DefaultRef>,
}

impl LazyDefaultRef {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            cell: std::sync::OnceLock::new(),
        }
    }

    pub fn get(&self) -> &DefaultRef {
        self.cell.get_or_init(|| resolve_default_ref(&self.dir))
    }
}

/// Outcome of the branch-state check for one slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchState {
    /// HEAD is an ancestor of D: nothing unmerged.
    NoOwnCommits,
    /// Content is on D, branch was pushed, and its remote branch is gone.
    MergedRemoteGone,
    /// N commits not on D and no upstream configured.
    Unpushed(u32),
    /// N commits not on D and the remote-tracking branch still exists.
    RemoteExists(u32),
    /// N commits not on D, remote branch gone, but content not on D.
    NotMerged(u32),
    /// Detached HEAD with N commits not on D.
    DetachedWithCommits(u32),
    /// No default ref could be resolved.
    DefaultUnknown,
}

impl BranchState {
    /// `true` when resetting the slot cannot lose work.
    pub fn is_available(&self) -> bool {
        matches!(self, Self::NoOwnCommits | Self::MergedRemoteGone)
    }

    /// Human-readable reason, relative to `default`.
    pub fn reason(&self, default: &DefaultRef) -> String {
        let d = default.name();
        match self {
            Self::NoOwnCommits => "no commits of its own".to_string(),
            Self::MergedRemoteGone => format!("merged into {d}, remote branch is gone"),
            Self::Unpushed(n) => format!(
                "{} ahead of {d}, no upstream configured \
                 (push with `-u` to enable merged-branch detection)",
                commits(*n)
            ),
            Self::RemoteExists(n) => {
                format!("{} ahead of {d}, remote branch still exists", commits(*n))
            }
            Self::NotMerged(n) => {
                format!("{} ahead of {d}, not merged", commits(*n))
            }
            Self::DetachedWithCommits(n) => {
                format!("detached HEAD with {} not on {d}", commits(*n))
            }
            Self::DefaultUnknown => "default branch could not be determined".to_string(),
        }
    }
}

fn commits(n: u32) -> String {
    if n == 1 {
        "1 commit".to_string()
    } else {
        format!("{n} commits")
    }
}

/// Run git in `dir`; `Some(stdout)` (trimmed) on success, `None` otherwise.
fn git_ok(dir: &Path, args: &[&str]) -> Option<String> {
    let out = git_cmd().arg("-C").arg(dir).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn ref_exists(dir: &Path, r: &str) -> bool {
    git_ok(dir, &["rev-parse", "--verify", "-q", r]).is_some()
}

/// Resolve the default ref once: `origin/HEAD`, else `origin/main`,
/// `origin/master`; local `main`/`master` only when there is no `origin`.
pub fn resolve_default_ref(dir: &Path) -> DefaultRef {
    if let Some(r) = git_ok(
        dir,
        &["symbolic-ref", "-q", "--short", "refs/remotes/origin/HEAD"],
    ) && !r.is_empty()
        && ref_exists(dir, &r)
    {
        return DefaultRef(Some(r));
    }
    for c in ["origin/main", "origin/master"] {
        if ref_exists(dir, &format!("refs/remotes/{c}")) {
            return DefaultRef(Some(c.to_string()));
        }
    }
    if git_ok(dir, &["remote", "get-url", "origin"]).is_none() {
        for c in ["main", "master"] {
            if ref_exists(dir, &format!("refs/heads/{c}")) {
                return DefaultRef(Some(c.to_string()));
            }
        }
    }
    DefaultRef(None)
}

fn count_ahead(slot: &Path, d: &str) -> u32 {
    git_ok(slot, &["rev-list", "--count", &format!("{d}..HEAD")])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Is the branch's content already on `d`? (`git cherry` for rebase merges,
/// `git merge-tree --write-tree` for squash merges.)
fn content_on_default(slot: &Path, d: &str) -> bool {
    if let Some(out) = git_ok(slot, &["cherry", d, "HEAD"])
        && !out.lines().any(|l| l.starts_with('+'))
    {
        return true;
    }
    let Some(out) = git_ok(slot, &["merge-tree", "--write-tree", d, "HEAD"]) else {
        return false;
    };
    let Some(result_tree) = out.lines().next() else {
        return false;
    };
    git_ok(slot, &["rev-parse", &format!("{d}^{{tree}}")]).as_deref() == Some(result_tree)
}

/// Evaluate the branch state of `slot` (whose checked-out branch is `branch`,
/// `None` if detached), cheapest check first with early return.
pub fn branch_state(slot: &Path, branch: Option<&str>, default: &DefaultRef) -> BranchState {
    let Some(d) = default.0.as_deref() else {
        return BranchState::DefaultUnknown;
    };

    let ancestor = git_cmd()
        .arg("-C")
        .arg(slot)
        .args(["merge-base", "--is-ancestor", "HEAD", d])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ancestor {
        return BranchState::NoOwnCommits;
    }

    let n = count_ahead(slot, d);
    let Some(branch) = branch else {
        return BranchState::DetachedWithCommits(n);
    };

    let remote = git_ok(slot, &["config", &format!("branch.{branch}.remote")]);
    let merge = git_ok(slot, &["config", &format!("branch.{branch}.merge")]);
    let (Some(remote), Some(merge)) = (remote, merge) else {
        return BranchState::Unpushed(n);
    };
    if remote == "." || remote.is_empty() || merge.is_empty() {
        return BranchState::Unpushed(n);
    }

    let merge_branch = merge.strip_prefix("refs/heads/").unwrap_or(&merge);
    if ref_exists(slot, &format!("refs/remotes/{remote}/{merge_branch}")) {
        return BranchState::RemoteExists(n);
    }

    if content_on_default(slot, d) {
        BranchState::MergedRemoteGone
    } else {
        BranchState::NotMerged(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d() -> DefaultRef {
        DefaultRef(Some("origin/main".to_string()))
    }

    #[test]
    fn availability() {
        assert!(BranchState::NoOwnCommits.is_available());
        assert!(BranchState::MergedRemoteGone.is_available());
        assert!(!BranchState::Unpushed(1).is_available());
        assert!(!BranchState::RemoteExists(1).is_available());
        assert!(!BranchState::NotMerged(1).is_available());
        assert!(!BranchState::DetachedWithCommits(1).is_available());
        assert!(!BranchState::DefaultUnknown.is_available());
    }

    #[test]
    fn unpushed_reason_mentions_commits_upstream_and_hint() {
        let r = BranchState::Unpushed(3).reason(&d());
        assert!(r.contains("3 commits ahead of origin/main"), "{r}");
        assert!(r.contains("no upstream configured"), "{r}");
        assert!(r.contains("-u"), "{r}");
    }

    #[test]
    fn singular_commit() {
        let r = BranchState::RemoteExists(1).reason(&d());
        assert!(r.contains("1 commit ahead"), "{r}");
        assert!(r.contains("remote branch still exists"), "{r}");
    }

    #[test]
    fn other_reasons() {
        assert!(
            BranchState::DefaultUnknown
                .reason(&DefaultRef(None))
                .contains("default branch could not be determined")
        );
        assert!(
            BranchState::NoOwnCommits
                .reason(&d())
                .contains("no commits of its own")
        );
        assert!(
            BranchState::MergedRemoteGone
                .reason(&d())
                .contains("remote branch is gone")
        );
    }

    fn git(dir: &Path, args: &[&str]) {
        let st = git_cmd()
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .expect("git");
        assert!(st.success(), "git {args:?} failed");
    }

    fn init_repo() -> tempfile::TempDir {
        let t = tempfile::TempDir::new().unwrap();
        git(t.path(), &["init", "-q", "-b", "main"]);
        git(t.path(), &["config", "user.email", "t@e.com"]);
        git(t.path(), &["config", "user.name", "T"]);
        git(t.path(), &["config", "commit.gpgsign", "false"]);
        git(t.path(), &["commit", "-q", "--allow-empty", "-m", "init"]);
        t
    }

    #[test]
    fn resolve_falls_back_to_local_main_without_remote() {
        let t = init_repo();
        assert_eq!(
            resolve_default_ref(t.path()),
            DefaultRef(Some("main".to_string()))
        );
    }

    #[test]
    fn resolve_none_when_origin_exists_without_refs() {
        let t = init_repo();
        git(t.path(), &["remote", "add", "origin", "/nonexistent"]);
        assert_eq!(resolve_default_ref(t.path()), DefaultRef(None));
    }

    #[test]
    fn resolve_prefers_origin_main_then_master_and_origin_head() {
        let t = init_repo();
        git(t.path(), &["remote", "add", "origin", "/nonexistent"]);
        git(
            t.path(),
            &["update-ref", "refs/remotes/origin/master", "HEAD"],
        );
        assert_eq!(
            resolve_default_ref(t.path()),
            DefaultRef(Some("origin/master".to_string()))
        );
        git(
            t.path(),
            &["update-ref", "refs/remotes/origin/main", "HEAD"],
        );
        assert_eq!(
            resolve_default_ref(t.path()),
            DefaultRef(Some("origin/main".to_string()))
        );
        git(
            t.path(),
            &["update-ref", "refs/remotes/origin/develop", "HEAD"],
        );
        git(
            t.path(),
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/develop",
            ],
        );
        assert_eq!(
            resolve_default_ref(t.path()),
            DefaultRef(Some("origin/develop".to_string()))
        );
    }

    #[test]
    fn branch_state_ancestry_unpushed_and_default_unknown() {
        let t = init_repo();
        let d = resolve_default_ref(t.path());
        assert_eq!(
            branch_state(t.path(), Some("main"), &d),
            BranchState::NoOwnCommits
        );
        assert_eq!(
            branch_state(t.path(), Some("main"), &DefaultRef(None)),
            BranchState::DefaultUnknown
        );
        git(t.path(), &["checkout", "-q", "-b", "feat"]);
        git(t.path(), &["commit", "-q", "--allow-empty", "-m", "x"]);
        assert_eq!(
            branch_state(t.path(), Some("feat"), &d),
            BranchState::Unpushed(1)
        );
        assert_eq!(
            branch_state(t.path(), None, &d),
            BranchState::DetachedWithCommits(1)
        );
    }
}
