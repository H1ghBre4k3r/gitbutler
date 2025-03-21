use crate::commit_engine::UpdatedReference;
use bstr::BString;
use gitbutler_oxidize::{ObjectIdExt, OidExt};
use gitbutler_stack::{CommitOrChangeId, VirtualBranchesState};
use gix::prelude::ObjectIdExt as _;
use gix::refs::transaction::PreviousValue;
use gix::revision::walk::Sorting;
use std::collections::BTreeMap;

use super::StackSegmentId;

/// Rewrite all references as mapped by their target in `refs_by_commit_id` so that those
/// pointing to `old` in `changed_commits` will then point to `new`.
/// Do the same for the virtual refs in `state` place information about all performed updates
/// in `updated_refs`.
/// `workspace_tip` is used, if present, to help build mappings from change-ids to commit-ids *if*
/// no target branch is available.
pub fn rewrite(
    repo: &gix::Repository,
    state: &mut VirtualBranchesState,
    workspace_tip: Option<gix::ObjectId>,
    mut refs_by_commit_id: gix::hashtable::HashMap<gix::ObjectId, Vec<gix::refs::FullName>>,
    changed_commits: impl IntoIterator<Item = (gix::ObjectId, gix::ObjectId)>,
    updated_refs: &mut Vec<UpdatedReference>,
    stack_segment: Option<&StackSegmentId>,
) -> anyhow::Result<()> {
    let mut ref_edits = Vec::new();
    let changed_commits: Vec<_> = changed_commits.into_iter().collect();
    let change_id_to_id_map = generate_change_ids_to_commit_mapping(repo, &*state, workspace_tip)?;
    let mut stacks_ordered: Vec<_> = state.branches.values_mut().collect();
    stacks_ordered.sort_by(|a, b| a.name.cmp(&b.name));
    for (old, new) in changed_commits {
        let old_git2 = old.to_git2();
        let mut already_updated_refs = Vec::<BString>::new();
        for stack in &mut stacks_ordered {
            if let Some(stack_segment) = stack_segment {
                if stack_segment.stack_id != stack.id {
                    continue; // Dont rewrite refs for other stacks
                }
            }
            if stack.head == old_git2 {
                stack.head = new.to_git2();
                stack.tree = new
                    .attach(repo)
                    .object()?
                    .into_commit()
                    .tree_id()?
                    .to_git2();
                updated_refs.push(UpdatedReference {
                    old_commit_id: old,
                    new_commit_id: new,
                    reference: but_core::Reference::Virtual(stack.name.clone()),
                });
            }
            let update_up_to_idx =
                stack_segment
                    .map(|s| s.segment_ref.as_ref())
                    .and_then(|up_to_ref| {
                        let short_name = up_to_ref.shorten();
                        stack
                            .heads
                            .iter()
                            .rev()
                            .enumerate()
                            .find_map(|(idx, h)| (h.name == short_name).then_some(idx))
                    });
            for (idx, branch) in stack.heads.iter_mut().rev().enumerate() {
                let id = match &mut branch.head() {
                    CommitOrChangeId::CommitId(id_hex) => {
                        let Some(id) = gix::ObjectId::from_hex(id_hex.as_bytes()).ok() else {
                            continue;
                        };
                        id
                    }
                    #[allow(deprecated)]
                    CommitOrChangeId::ChangeId(change_id) => {
                        let Some(id) = change_id_to_id_map.get(change_id) else {
                            continue;
                        };
                        *id
                    }
                };
                if id == old {
                    if update_up_to_idx.is_some() && Some(idx) > update_up_to_idx {
                        // Make sure the actual refs also don't update (later)
                        already_updated_refs.push(format!("refs/heads/{}", branch.name()).into());
                        continue;
                    }
                    if let Some(full_refname) =
                        branch.set_head(CommitOrChangeId::CommitId(new.to_string()), repo)?
                    {
                        already_updated_refs.push(full_refname)
                    }
                    updated_refs.push(UpdatedReference {
                        old_commit_id: old,
                        new_commit_id: new,
                        reference: but_core::Reference::Virtual(branch.name().clone()),
                    });
                }
            }
        }

        let Some(refs_to_rewrite) = refs_by_commit_id.remove(&old) else {
            continue;
        };

        for name in refs_to_rewrite {
            if already_updated_refs.iter().any(|r| name.as_bstr() == r) {
                continue;
            }
            use gix::refs::{
                Target,
                transaction::{Change, LogChange, RefEdit, RefLog},
            };
            updated_refs.push(UpdatedReference {
                old_commit_id: old,
                new_commit_id: new,
                reference: but_core::Reference::Git(name.clone()),
            });
            ref_edits.push(RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: "Created or amended commit".into(),
                    },
                    expected: PreviousValue::ExistingMustMatch(Target::Object(old)),
                    new: Target::Object(new),
                },
                name,
                deref: false,
            });
        }
    }
    repo.edit_references(ref_edits)?;
    Ok(())
}

fn generate_change_ids_to_commit_mapping(
    repo: &gix::Repository,
    vb: &VirtualBranchesState,
    workspace_tip: Option<gix::ObjectId>,
) -> anyhow::Result<BTreeMap<String, gix::ObjectId>> {
    let cache = repo.commit_graph_if_enabled()?;
    let mut graph = repo.revision_graph(cache.as_ref());
    let default_target_tip = vb
        .default_target
        .as_ref()
        .map(|target| -> anyhow::Result<_> {
            let r = repo.find_reference(&target.branch.to_string())?;
            Ok(r.try_id())
        })
        .and_then(Result::ok)
        .flatten();

    let mut out = BTreeMap::new();
    let merge_base = if default_target_tip.is_none() {
        let Some(workspace_tip) = workspace_tip else {
            return Ok(out);
        };
        let workspace_commit = workspace_tip
            .attach(repo)
            .object()?
            .into_commit()
            .decode()?
            .to_owned();
        if workspace_commit.parents.len() < 2 {
            None
        } else {
            Some(repo.merge_base_octopus(workspace_commit.parents)?)
        }
    } else {
        None
    };
    for stack in vb.branches.values().filter(|b| b.in_workspace) {
        let stack_tip = stack.head.to_gix();
        for info in stack_tip
            .attach(repo)
            .ancestors()
            .with_boundary(match default_target_tip {
                Some(target_tip) => {
                    Some(repo.merge_base_with_graph(stack_tip, target_tip, &mut graph)?)
                }
                None => merge_base,
            })
            .sorting(Sorting::BreadthFirst)
            .all()?
            .filter_map(Result::ok)
        {
            let Some(headers) = but_core::Commit::from_id(info.id.attach(repo))?.headers() else {
                continue;
            };
            out.insert(headers.change_id, info.id);
        }
    }
    Ok(out)
}
