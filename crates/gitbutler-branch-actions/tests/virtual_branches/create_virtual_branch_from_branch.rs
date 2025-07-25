use gitbutler_branch::BranchCreateRequest;
use gitbutler_reference::LocalRefname;

use super::*;

#[test]
fn integration() {
    let Test { repo, ctx, .. } = &Test::default();

    gitbutler_branch_actions::set_base_branch(
        ctx,
        &"refs/remotes/origin/master".parse().unwrap(),
        false,
        ctx.project().exclusive_worktree_access().write_permission(),
    )
    .unwrap();

    let branch_name = {
        // make a remote branch

        let stack_entry = gitbutler_branch_actions::create_virtual_branch(
            ctx,
            &BranchCreateRequest::default(),
            ctx.project().exclusive_worktree_access().write_permission(),
        )
        .unwrap();

        std::fs::write(repo.path().join("file.txt"), "first\n").unwrap();
        gitbutler_branch_actions::create_commit(ctx, stack_entry.id, "first", None).unwrap();
        gitbutler_branch_actions::stack::push_stack(ctx, stack_entry.id, false, None).unwrap();

        let branch = gitbutler_branch_actions::list_virtual_branches(ctx)
            .unwrap()
            .branches
            .into_iter()
            .find(|branch| branch.id == stack_entry.id)
            .unwrap();

        let name = branch
            .series
            .first()
            .unwrap()
            .as_ref()
            .unwrap()
            .upstream_reference
            .as_ref()
            .unwrap();

        gitbutler_branch_actions::unapply_stack(ctx, stack_entry.id, Vec::new()).unwrap();

        Refname::from_str(name).unwrap()
    };

    // checkout a existing remote branch
    let branch_id = gitbutler_branch_actions::create_virtual_branch_from_branch(
        ctx,
        &branch_name,
        None,
        Some(123),
    )
    .unwrap();

    {
        // add a commit
        std::fs::write(repo.path().join("file.txt"), "first\nsecond").unwrap();

        gitbutler_branch_actions::create_commit(ctx, branch_id, "second", None).unwrap();
    }

    {
        // meanwhile, there is a new commit on master
        repo.checkout(&"refs/heads/master".parse().unwrap());
        std::fs::write(repo.path().join("another.txt"), "").unwrap();
        repo.commit_all("another");
        repo.push_branch(&"refs/heads/master".parse().unwrap());
        repo.checkout(&"refs/heads/gitbutler/workspace".parse().unwrap());
    }

    {
        // merge branch into master
        gitbutler_branch_actions::stack::push_stack(ctx, branch_id, false, None).unwrap();

        let branch = gitbutler_branch_actions::list_virtual_branches(ctx)
            .unwrap()
            .branches
            .into_iter()
            .find(|branch| branch.id == branch_id)
            .unwrap();

        assert!(branch.series[0].clone().unwrap().patches[0].is_local_and_remote);
        assert!(!branch.series[0].clone().unwrap().patches[0].is_integrated);
        assert!(branch.series[0].clone().unwrap().patches[1].is_local_and_remote);
        assert!(!branch.series[0].clone().unwrap().patches[1].is_integrated);

        repo.rebase_and_merge(&branch_name);
    }

    {
        // should mark commits as integrated
        gitbutler_branch_actions::fetch_from_remotes(ctx, None).unwrap();

        let branch = gitbutler_branch_actions::list_virtual_branches(ctx)
            .unwrap()
            .branches
            .into_iter()
            .find(|branch| branch.id == branch_id)
            .unwrap();

        assert_eq!(
            branch.series.first().unwrap().clone().unwrap().pr_number,
            Some(123)
        );

        assert!(branch.series[0].clone().unwrap().patches[0].is_local_and_remote);
        assert!(branch.series[0].clone().unwrap().patches[0].is_integrated);
        assert!(branch.series[0].clone().unwrap().patches[1].is_local_and_remote);
        assert!(branch.series[0].clone().unwrap().patches[1].is_integrated);
    }
}

#[test]
fn no_conflicts() {
    let Test { repo, ctx, .. } = &Test::default();

    {
        // create a remote branch
        let branch_name: LocalRefname = "refs/heads/branch".parse().unwrap();
        repo.checkout(&branch_name);
        fs::write(repo.path().join("file.txt"), "first").unwrap();
        repo.commit_all("first");
        repo.push_branch(&branch_name);
        repo.checkout(&"refs/heads/master".parse().unwrap());
    }

    gitbutler_branch_actions::set_base_branch(
        ctx,
        &"refs/remotes/origin/master".parse().unwrap(),
        false,
        ctx.project().exclusive_worktree_access().write_permission(),
    )
    .unwrap();

    let list_result = gitbutler_branch_actions::list_virtual_branches(ctx).unwrap();
    let branches = list_result.branches;

    assert!(branches.is_empty());

    let branch_id = gitbutler_branch_actions::create_virtual_branch_from_branch(
        ctx,
        &"refs/remotes/origin/branch".parse().unwrap(),
        None,
        None,
    )
    .unwrap();

    let list_result = gitbutler_branch_actions::list_virtual_branches(ctx).unwrap();
    let branches = list_result.branches;
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].id, branch_id);
    assert_eq!(branches[0].series[0].clone().unwrap().patches.len(), 1);
    assert_eq!(
        branches[0].series[0].clone().unwrap().patches[0].description,
        "first"
    );
}

#[test]
fn conflicts_with_uncommited() {
    let Test { repo, ctx, .. } = &Test::default();

    {
        // create a remote branch
        let branch_name: LocalRefname = "refs/heads/branch".parse().unwrap();
        repo.checkout(&branch_name);
        fs::write(repo.path().join("file.txt"), "first").unwrap();
        repo.commit_all("first");
        repo.push_branch(&branch_name);
        repo.checkout(&"refs/heads/master".parse().unwrap());
    }

    gitbutler_branch_actions::set_base_branch(
        ctx,
        &"refs/remotes/origin/master".parse().unwrap(),
        false,
        ctx.project().exclusive_worktree_access().write_permission(),
    )
    .unwrap();

    // create a local branch that conflicts with remote
    {
        std::fs::write(repo.path().join("file.txt"), "conflict").unwrap();

        let list_result = gitbutler_branch_actions::list_virtual_branches(ctx).unwrap();
        let branches = list_result.branches;
        assert_eq!(branches.len(), 1);
    };

    // branch should be created unapplied, because of the conflict

    let new_branch_id = gitbutler_branch_actions::create_virtual_branch_from_branch(
        ctx,
        &"refs/remotes/origin/branch".parse().unwrap(),
        None,
        None,
    )
    .unwrap();
    let new_branch = gitbutler_branch_actions::list_virtual_branches(ctx)
        .unwrap()
        .branches
        .into_iter()
        .find(|branch| branch.id == new_branch_id)
        .unwrap();
    assert_eq!(new_branch_id, new_branch.id);
    assert_eq!(new_branch.series[0].clone().unwrap().patches.len(), 1);
    assert!(new_branch.upstream.is_some());
}

#[test]
fn conflicts_with_commited() {
    let Test { repo, ctx, .. } = &Test::default();

    {
        // create a remote branch
        let branch_name: LocalRefname = "refs/heads/branch".parse().unwrap();
        repo.checkout(&branch_name);
        fs::write(repo.path().join("file.txt"), "first").unwrap();
        repo.commit_all("first");
        repo.push_branch(&branch_name);
        repo.checkout(&"refs/heads/master".parse().unwrap());
    }

    gitbutler_branch_actions::set_base_branch(
        ctx,
        &"refs/remotes/origin/master".parse().unwrap(),
        false,
        ctx.project().exclusive_worktree_access().write_permission(),
    )
    .unwrap();

    // create a local branch that conflicts with remote
    {
        std::fs::write(repo.path().join("file.txt"), "conflict").unwrap();

        let list_result = gitbutler_branch_actions::list_virtual_branches(ctx).unwrap();
        let branches = list_result.branches;
        assert_eq!(branches.len(), 1);

        gitbutler_branch_actions::create_commit(ctx, branches[0].id, "hej", None).unwrap();
    };

    // branch should be created unapplied, because of the conflict

    let new_branch_id = gitbutler_branch_actions::create_virtual_branch_from_branch(
        ctx,
        &"refs/remotes/origin/branch".parse().unwrap(),
        None,
        None,
    )
    .unwrap();
    let new_branch = gitbutler_branch_actions::list_virtual_branches(ctx)
        .unwrap()
        .branches
        .into_iter()
        .find(|branch| branch.id == new_branch_id)
        .unwrap();
    assert_eq!(new_branch_id, new_branch.id);
    assert_eq!(new_branch.series[0].clone().unwrap().patches.len(), 1);
    assert!(new_branch.upstream.is_some());
}

#[test]
fn from_default_target() {
    let Test { ctx, .. } = &Test::default();

    gitbutler_branch_actions::set_base_branch(
        ctx,
        &"refs/remotes/origin/master".parse().unwrap(),
        false,
        ctx.project().exclusive_worktree_access().write_permission(),
    )
    .unwrap();

    // branch should be created unapplied, because of the conflict

    assert_eq!(
        gitbutler_branch_actions::create_virtual_branch_from_branch(
            ctx,
            &"refs/remotes/origin/master".parse().unwrap(),
            None,
            None,
        )
        .unwrap_err()
        .to_string(),
        "cannot create a branch from default target"
    );
}

#[test]
fn from_non_existent_branch() {
    let Test { ctx, .. } = &Test::default();

    gitbutler_branch_actions::set_base_branch(
        ctx,
        &"refs/remotes/origin/master".parse().unwrap(),
        false,
        ctx.project().exclusive_worktree_access().write_permission(),
    )
    .unwrap();

    // branch should be created unapplied, because of the conflict

    assert_eq!(
        gitbutler_branch_actions::create_virtual_branch_from_branch(
            ctx,
            &"refs/remotes/origin/branch".parse().unwrap(),
            None,
            None,
        )
        .unwrap_err()
        .to_string(),
        "branch refs/remotes/origin/branch was not found"
    );
}

#[test]
fn from_state_remote_branch() {
    let Test { repo, ctx, .. } = &Test::default();

    {
        // create a remote branch
        let branch_name: LocalRefname = "refs/heads/branch".parse().unwrap();
        repo.checkout(&branch_name);
        fs::write(repo.path().join("file.txt"), "branch commit").unwrap();
        repo.commit_all("branch commit");
        repo.push_branch(&branch_name);
        repo.checkout(&"refs/heads/master".parse().unwrap());

        // make remote branch stale
        std::fs::write(repo.path().join("antoher_file.txt"), "master commit").unwrap();
        repo.commit_all("master commit");
        repo.push();
    }

    gitbutler_branch_actions::set_base_branch(
        ctx,
        &"refs/remotes/origin/master".parse().unwrap(),
        false,
        ctx.project().exclusive_worktree_access().write_permission(),
    )
    .unwrap();

    let branch_id = gitbutler_branch_actions::create_virtual_branch_from_branch(
        ctx,
        &"refs/remotes/origin/branch".parse().unwrap(),
        None,
        None,
    )
    .unwrap();

    let list_result = gitbutler_branch_actions::list_virtual_branches(ctx).unwrap();
    let branches = list_result.branches;
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].id, branch_id);
    assert_eq!(branches[0].series[0].clone().unwrap().patches.len(), 1);
    assert!(branches[0].files.is_empty());
    assert_eq!(
        branches[0].series[0].clone().unwrap().patches[0].description,
        "branch commit"
    );
}

#[cfg(test)]
mod conflict_cases {
    use bstr::ByteSlice as _;
    use gitbutler_testsupport::testing_repository::{
        assert_commit_tree_matches, assert_tree_matches,
    };

    use super::*;

    /// Same setup as above, but with fearless rebasing, so we should end up
    /// with some conflicted commits.
    #[test]
    fn apply_mergable_but_not_rebasable_branch_with_fearless() {
        let Test { repo, ctx, .. } = &Test::default();

        let git_repo = &repo.local_repo;
        let signature = git2::Signature::now("caleb", "caleb@gitbutler.com").unwrap();

        let head_commit = git_repo.head().unwrap().peel_to_commit().unwrap();

        git_repo
            .reference("refs/remotes/origin/master", head_commit.id(), true, ":D")
            .unwrap();

        gitbutler_branch_actions::set_base_branch(
            ctx,
            &"refs/remotes/origin/master".parse().unwrap(),
            false,
            ctx.project().exclusive_worktree_access().write_permission(),
        )
        .unwrap();

        // Make A and B and unapply them.
        fs::write(repo.path().join("foo.txt"), "a").unwrap();
        repo.commit_all("A");
        fs::remove_file(repo.path().join("foo.txt")).unwrap();
        fs::write(repo.path().join("bar.txt"), "b").unwrap();
        repo.commit_all("B");

        let list_result = gitbutler_branch_actions::list_virtual_branches(ctx).unwrap();
        let branches = list_result.branches;
        let branch = branches[0].clone();

        let branch_refname =
            gitbutler_branch_actions::unapply_stack(ctx, branch.id, Vec::new()).unwrap();

        // Make X and set base branch to X
        let mut tree_builder = git_repo
            .treebuilder(Some(&git_repo.head().unwrap().peel_to_tree().unwrap()))
            .unwrap();
        let blob_oid = git_repo.blob("x".as_bytes()).unwrap();
        tree_builder
            .insert("foo.txt", blob_oid, git2::FileMode::Blob.into())
            .unwrap();

        git_repo
            .commit(
                Some("refs/remotes/origin/master"),
                &signature,
                &signature,
                "X",
                &git_repo.find_tree(tree_builder.write().unwrap()).unwrap(),
                &[&head_commit],
            )
            .unwrap();

        gitbutler_branch_actions::integrate_upstream(ctx, &[], None).unwrap();

        // Apply B

        gitbutler_branch_actions::create_virtual_branch_from_branch(
            ctx,
            &Refname::from_str(&branch_refname).unwrap(),
            None,
            None,
        )
        .unwrap();

        // We should see a merge commit
        let list_result = gitbutler_branch_actions::list_virtual_branches(ctx).unwrap();
        let branches = list_result.branches;
        let branch = branches[0].clone();

        assert_eq!(
            branch.series[0].clone().unwrap().patches.len(),
            2,
            "Should have B' and A'"
        );

        assert_eq!(
            branch.series[0].clone().unwrap().patches[0]
                .description
                .to_str()
                .unwrap(),
            "B"
        );
        assert!(branch.series[0].clone().unwrap().patches[0].conflicted);
        let tree = repo
            .find_commit(branch.series[0].clone().unwrap().patches[0].id)
            .unwrap()
            .tree()
            .unwrap();
        assert_eq!(tree.len(), 6, "Five trees and the readme");
        assert_tree_matches(
            git_repo,
            &tree,
            &[
                (".auto-resolution/foo.txt", b"x"), // Has "ours" foo content
                (".auto-resolution/bar.txt", b"b"), // Has unconflicted "theirs" content
                (".conflict-base-0/foo.txt", b"a"), // A is base
                (".conflict-side-0/foo.txt", b"x"), // "Ours" is A'
                (".conflict-side-1/bar.txt", b"b"), // "Theirs" is B
            ],
        );

        assert_eq!(
            branch.series[0].clone().unwrap().patches[1]
                .description
                .to_str()
                .unwrap(),
            "A"
        );
        assert!(branch.series[0].clone().unwrap().patches[1].conflicted);
        assert_commit_tree_matches(
            git_repo,
            &repo
                .find_commit(branch.series[0].clone().unwrap().patches[1].id)
                .unwrap(),
            &[
                (".auto-resolution/foo.txt", b"x"), // Auto-resolves to X
                (".conflict-side-0/foo.txt", b"x"), // "Ours" is X
                (".conflict-side-1/foo.txt", b"a"), // "Theirs" is A
            ],
        );
    }
}
