<script lang="ts">
	import ReduxResult from '$components/ReduxResult.svelte';
	import CommitContextMenu, {
		type CommitMenuContext
	} from '$components/v3/CommitContextMenu.svelte';
	import CommitDetails from '$components/v3/CommitDetails.svelte';
	import CommitMessageEditor from '$components/v3/CommitMessageEditor.svelte';
	import CommitTitle from '$components/v3/CommitTitle.svelte';
	import Drawer from '$components/v3/Drawer.svelte';
	import KebabButton from '$components/v3/KebabButton.svelte';
	import { isLocalAndRemoteCommit } from '$components/v3/lib';
	import { isCommit } from '$lib/branches/v3';
	import { type CommitKey } from '$lib/commits/commit';
	import { DefaultForgeFactory } from '$lib/forge/forgeFactory.svelte';
	import { ModeService } from '$lib/mode/modeService';
	import { showToast } from '$lib/notifications/toasts';
	import { StackService } from '$lib/stacks/stackService.svelte';
	import { UiState } from '$lib/state/uiState.svelte';
	import { TestId } from '$lib/testing/testIds';
	import { splitMessage } from '$lib/utils/commitMessage';
	import { inject } from '@gitbutler/shared/context';
	import { getContext, maybeGetContext } from '@gitbutler/shared/context';
	import AsyncButton from '@gitbutler/ui/AsyncButton.svelte';
	import Button from '@gitbutler/ui/Button.svelte';
	import type { TargetType } from '$lib/intelligentScrolling/service';
	import type { Snippet } from 'svelte';

	type Props = {
		projectId: string;
		stackId: string;
		commitKey: CommitKey;
		active?: boolean;
		draggableFiles: boolean;
		collapsible?: boolean;
		scrollToType?: TargetType;
		scrollToId?: string;
		grow?: boolean;
		resizer?: Snippet<[{ element: HTMLDivElement; collapsed?: boolean }]>;
		ontoggle?: (collapsed: boolean) => void;
		onerror: (err: unknown) => void;
		onclose?: () => void;
	};

	const {
		projectId,
		stackId,
		commitKey,
		collapsible,
		scrollToId,
		scrollToType,
		grow,
		resizer,
		ontoggle,
		onerror,
		onclose
	}: Props = $props();

	const [stackService, uiState] = inject(StackService, UiState);

	const forge = getContext(DefaultForgeFactory);
	const modeService = maybeGetContext(ModeService);
	const stackState = $derived(uiState.stack(stackId));
	const projectState = $derived(uiState.project(projectId));
	const selected = $derived(stackState.selection.get());
	const branchName = $derived(selected.current?.branchName);

	const commitResult = $derived(
		commitKey.upstream
			? stackService.upstreamCommitById(projectId, commitKey)
			: stackService.commitById(projectId, commitKey)
	);

	const [updateCommitMessage, messageUpdateResult] = stackService.updateCommitMessage;

	type Mode = 'view' | 'edit';

	function setMode(newMode: Mode) {
		switch (newMode) {
			case 'edit':
				projectState.exclusiveAction.set({
					type: 'edit-commit-message',
					commitId: commitKey.commitId
				});
				break;
			case 'view':
				projectState.exclusiveAction.set(undefined);
				break;
		}
	}

	const parsedMessage = $derived(
		commitResult.current.data ? splitMessage(commitResult.current.data.message) : undefined
	);

	function combineParts(title?: string, description?: string): string {
		if (!title) {
			return '';
		}
		if (description) {
			return `${title}\n\n${description}`;
		}
		return title;
	}

	async function saveCommitMessage(title: string, description: string) {
		const commitMessage = combineParts(title, description);
		if (!branchName) {
			throw new Error('No branch selected!');
		}
		if (!commitMessage) {
			showToast({ message: 'Commit message is required', style: 'error' });
			return;
		}

		const newCommitId = await updateCommitMessage({
			projectId,
			stackId,
			commitId: commitKey.commitId,
			message: commitMessage
		});

		uiState.stack(stackId).selection.set({ branchName, commitId: newCommitId });
		setMode('view');
	}

	let commitMenuContext = $state<CommitMenuContext>();

	async function handleUncommit() {
		if (!branchName) return;
		await stackService.uncommit({ projectId, stackId, branchName, commitId: commitKey.commitId });
	}

	function canEdit() {
		return modeService !== undefined;
	}

	async function editPatch() {
		if (!canEdit()) return;
		await modeService!.enterEditMode(commitKey.commitId, stackId);
	}

	function cancelEdit() {
		setMode('view');
	}
</script>

<ReduxResult {stackId} {projectId} result={commitResult.current} {onerror}>
	{#snippet children(commit, env)}
		{@const isConflicted = isCommit(commit) && commit.hasConflicts}

		<Drawer
			{collapsible}
			testId={TestId.CommitDrawer}
			bottomBorder={!!resizer || !collapsible}
			{scrollToId}
			{scrollToType}
			{ontoggle}
			{onclose}
			{resizer}
			{grow}
		>
			{#snippet header()}
				<CommitTitle
					truncate
					commitMessage={commit.message}
					className="text-14 text-semibold text-body"
				/>
			{/snippet}

			{#snippet kebabMenu(header)}
				{@const data = isLocalAndRemoteCommit(commit)
					? {
							stackId,
							commitId: commit.id,
							commitMessage: commit.message,
							commitStatus: commit.state.type,
							commitUrl: forge.current.commitUrl(commit.id),
							onUncommitClick: () => handleUncommit(),
							onEditMessageClick: () => setMode('edit'),
							onPatchEditClick: () => editPatch()
						}
					: undefined}
				{#if data}
					<KebabButton
						contextElement={header}
						onclick={(element) => (commitMenuContext = { data, position: { element } })}
						oncontext={(coords) => (commitMenuContext = { data, position: { coords } })}
						activated={!!commitMenuContext?.position.element}
					/>
				{/if}
			{/snippet}

			<div class="commit-view">
				{#if projectState.exclusiveAction.current?.type === 'edit-commit-message' && projectState.exclusiveAction.current.commitId === commit.id}
					<div
						class="edit-commit-view"
						data-testid={TestId.EditCommitMessageBox}
						class:no-paddings={uiState.global.useFloatingCommitBox.current}
					>
						<CommitMessageEditor
							noPadding
							projectId={env.projectId}
							stackId={env.stackId}
							action={({ title, description }) => saveCommitMessage(title, description)}
							actionLabel="Save changes"
							onCancel={cancelEdit}
							floatingBoxHeader="Edit commit message"
							loading={messageUpdateResult.current.isLoading}
							existingCommitId={commit.id}
							title={parsedMessage?.title || ''}
							description={parsedMessage?.description || ''}
						/>
					</div>
				{:else}
					<!-- <CommitTitle commitMessage={commit.message} className="text-14 text-semibold text-body" /> -->
					<CommitDetails {commit}>
						<Button
							testId={TestId.CommitDrawerActionEditMessage}
							size="tag"
							kind="outline"
							icon="edit-small"
							onclick={() => {
								setMode('edit');
							}}
						>
							Edit message
						</Button>

						<AsyncButton
							testId={TestId.CommitDrawerActionUncommit}
							size="tag"
							kind="outline"
							icon="undo-small"
							action={async () => await handleUncommit()}
						>
							Uncommit
						</AsyncButton>

						{#if isConflicted}
							<AsyncButton
								size="tag"
								kind="solid"
								style="error"
								action={editPatch}
								icon="warning-small"
							>
								Resolve conflicts
							</AsyncButton>
						{:else}
							<AsyncButton size="tag" kind="outline" action={editPatch}>Edit commit</AsyncButton>
						{/if}
					</CommitDetails>
				{/if}
			</div>
		</Drawer>
	{/snippet}
</ReduxResult>

{#if commitMenuContext}
	<CommitContextMenu {projectId} bind:context={commitMenuContext} />
{/if}

<style>
	.commit-view {
		display: flex;
		position: relative;
		flex: 1;
		flex-direction: column;
		height: 100%;
		padding: 14px;
		gap: 14px;
		background-color: var(--clr-bg-1);
	}

	.edit-commit-view {
		display: flex;
		flex-direction: column;

		&.no-paddings {
			margin: -14px;
		}
	}
</style>
