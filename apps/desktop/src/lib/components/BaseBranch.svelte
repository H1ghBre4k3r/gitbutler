<script lang="ts">
	import IntegrateUpstreamModal from './IntegrateUpstreamModal.svelte';
	import { Project } from '$lib/backend/projects';
	import { BaseBranchService } from '$lib/baseBranch/baseBranchService';
	import CommitAction from '$lib/commit/CommitAction.svelte';
	import CommitCard from '$lib/commit/CommitCard.svelte';
	import { transformAnyCommit } from '$lib/commitLines/transformers';
	import { projectMergeUpstreamWarningDismissed } from '$lib/config/config';
	import { getGitHost } from '$lib/gitHost/interface/gitHost';
	import { ModeService } from '$lib/modes/service';
	import { showInfo } from '$lib/notifications/toasts';
	import InfoMessage from '$lib/shared/InfoMessage.svelte';
	import { groupByCondition } from '$lib/utils/array';
	import { getContext } from '$lib/utils/context';
	import { BranchController } from '$lib/vbranches/branchController';
	import Button from '@gitbutler/ui/Button.svelte';
	import Checkbox from '@gitbutler/ui/Checkbox.svelte';
	import Modal from '@gitbutler/ui/Modal.svelte';
	import LineGroup from '@gitbutler/ui/commitLines/LineGroup.svelte';
	import { LineManagerFactory, LineSpacer } from '@gitbutler/ui/commitLines/lineManager';
	import { tick } from 'svelte';
	import type { BaseBranch } from '$lib/baseBranch/baseBranch';

	interface Props {
		base: BaseBranch;
	}

	const { base }: Props = $props();

	const resetBaseTo = {
		local: {
			title: 'Push local changes',
			header:
				'You are about to reset the upstream target branch to the local branch. This will lose all the remote changes not in the local branch.',
			content: 'You will force-push your local changes to the remote branch.',
			tooltip:
				'Resets the upstream branch to the local target branch. Will lose all the remote changes not in the local branch.',
			color: 'pop',
			handler: handlePushLocalChanges,
			action: pushBaseBranch
		},
		remote: {
			title: 'Discard local changes',
			header:
				'You are about to reset the target branch to the remote branch. This will lose all the changes ahead of the remote branch.',
			content: 'You are about to hard reset your local base branch to the remote branch',
			tooltip: `Choose how to integrate the commits from ${base.branchName} into the base of all applied virtual branches`,
			color: 'warning',
			handler: handleMergeUpstream,
			action: updateBaseBranch
		}
	} as const;

	type ResetBaseStrategy = keyof typeof resetBaseTo;

	const baseBranchService = getContext(BaseBranchService);
	const branchController = getContext(BranchController);
	const modeService = getContext(ModeService);
	const gitHost = getGitHost();
	const project = getContext(Project);
	const lineManagerFactory = getContext(LineManagerFactory);

	const mode = $derived(modeService.mode);
	const mergeUpstreamWarningDismissed = $derived(
		projectMergeUpstreamWarningDismissed(branchController.projectId)
	);

	let baseBranchIsUpdating = $state<boolean>(false);
	const baseBranchConflicted = $derived(base.conflicted);
	let updateTargetModal = $state<Modal>();
	let resetBaseStrategy = $state<ResetBaseStrategy | undefined>(undefined);
	let confirmResetModal = $state<Modal>();
	const confirmResetModalOpen = $derived(!!confirmResetModal?.imports.open);
	let integrateUpstreamModal = $state<IntegrateUpstreamModal>();
	const integrateUpstreamModalOpen = $derived(!!integrateUpstreamModal?.imports.open);
	let mergeUpstreamWarningDismissedCheckbox = $state<boolean>(false);

	const pushButtonTooltip = $derived.by(() => {
		if (onlyLocalAhead) return 'Push your local changes to upstream';
		if (base.conflicted) return 'Cannot push while there are conflicts';
		return resetBaseTo.local.tooltip;
	});

	const multiple = $derived(
		base ? base.upstreamCommits.length > 1 || base.upstreamCommits.length === 0 : false
	);

	const onlyLocalAhead = $derived(
		base.diverged && base.divergedBehind.length === 0 && base.divergedAhead.length > 0
	);

	const { satisfied: commitsAhead, rest: localAndRemoteCommits } = $derived(
		groupByCondition(base.recentCommits, (c) => base.divergedAhead.includes(c.id))
	);

	const mappedRemoteCommits = $derived(
		base.upstreamCommits.length > 0
			? [...base.upstreamCommits.map(transformAnyCommit), { id: LineSpacer.Remote }]
			: []
	);

	const mappedLocalCommits = $derived.by(() => {
		if (!base.diverged) return [];
		return commitsAhead.length > 0
			? [...commitsAhead.map(transformAnyCommit), { id: LineSpacer.Local }]
			: [];
	});

	const mappedLocalAndRemoteCommits = $derived.by(() => {
		return localAndRemoteCommits.length > 0
			? [...localAndRemoteCommits.map(transformAnyCommit), { id: LineSpacer.LocalAndRemote }]
			: [];
	});

	const lineManager = $derived(
		lineManagerFactory.build(
			{
				remoteCommits: mappedRemoteCommits,
				localCommits: mappedLocalCommits,
				localAndRemoteCommits: mappedLocalAndRemoteCommits,
				integratedCommits: []
			},
			true
		)
	);

	async function updateBaseBranch() {
		baseBranchIsUpdating = true;
		const infoText = await branchController.updateBaseBranch();
		if (infoText) {
			showInfo('Stashed conflicting branches', infoText);
		}
		await tick();
		baseBranchIsUpdating = false;
	}

	async function handleMergeUpstream() {
		if (project.succeedingRebases && !onlyLocalAhead) {
			integrateUpstreamModal?.show();
			return;
		}

		if (base.diverged) {
			await confirmResetBranch('remote');
			return;
		}

		if ($mergeUpstreamWarningDismissed) {
			updateBaseBranch();
			return;
		}
		updateTargetModal?.show();
	}

	async function pushBaseBranch() {
		baseBranchIsUpdating = true;
		await baseBranchService.push(!onlyLocalAhead);
		await tick();
		await baseBranchService.refresh();
		baseBranchIsUpdating = false;
	}

	async function confirmResetBranch(strategy: ResetBaseStrategy) {
		resetBaseStrategy = strategy;
		await tick();
		confirmResetModal?.show();
	}

	async function handlePushLocalChanges() {
		if (onlyLocalAhead) {
			await pushBaseBranch();
			return;
		}
		await confirmResetBranch('local');
	}
</script>

{#if base.diverged}
	<div class="message-wrapper">
		{#if !onlyLocalAhead}
			<InfoMessage style="warning" filled outlined={false}>
				<svelte:fragment slot="content">
					Your local target branch has diverged from upstream.
					<br />
					Target branch is
					<b>
						{`ahead by ${base.divergedAhead.length}`}
					</b>
					commits and
					<b>
						{`behind by ${base.divergedBehind.length}`}
					</b>
					commits
				</svelte:fragment>
			</InfoMessage>
		{:else}
			<InfoMessage style="neutral" filled outlined={false}>
				<svelte:fragment slot="content">
					Your local target branch is
					<b>
						{`ahead by ${base.divergedAhead.length}`}
					</b>
					commits
				</svelte:fragment>
			</InfoMessage>
		{/if}
	</div>
{/if}

{#if !base.diverged && base.upstreamCommits.length > 0}
	<div class="header-wrapper">
		<div class="info-text text-13">
			There {multiple ? 'are' : 'is'}
			{base.upstreamCommits.length} unmerged upstream
			{multiple ? 'commits' : 'commit'}
		</div>

		<Button
			style="pop"
			kind="solid"
			tooltip={`Merges the commits from ${base.branchName} into the base of all applied virtual branches`}
			disabled={$mode?.type !== 'OpenWorkspace' || integrateUpstreamModalOpen}
			loading={integrateUpstreamModalOpen}
			onclick={handleMergeUpstream}
		>
			Merge into common base
		</Button>
	</div>
{/if}

<div class="wrapper">
	<!-- UPSTREAM COMMITS -->
	{#if base.upstreamCommits?.length > 0}
		<div>
			{#each base.upstreamCommits as commit, index}
				<CommitCard
					{commit}
					first={index === 0}
					last={index === base.upstreamCommits.length - 1}
					isUnapplied={true}
					commitUrl={$gitHost?.commitUrl(commit.id)}
					type="remote"
				>
					{#snippet lines(topHeightPx)}
						<LineGroup lineGroup={lineManager.get(commit.id)} {topHeightPx} />
					{/snippet}
				</CommitCard>
			{/each}
		</div>

		{#if base.diverged}
			<CommitAction backgroundColor={false}>
				{#snippet lines()}
					<LineGroup lineGroup={lineManager.get(LineSpacer.Remote)} topHeightPx={0} />
				{/snippet}
				{#snippet action()}
					<Button
						wide
						icon="warning"
						kind="solid"
						style={resetBaseTo.remote.color}
						tooltip={resetBaseTo.remote.tooltip}
						loading={baseBranchIsUpdating || integrateUpstreamModalOpen}
						disabled={$mode?.type !== 'OpenWorkspace' ||
							baseBranchIsUpdating ||
							integrateUpstreamModalOpen}
						onclick={resetBaseTo.remote.handler}
					>
						Integrate upstream changes
					</Button>
				{/snippet}
			</CommitAction>
		{/if}
	{/if}

	<!-- DIVERGED (LOCAL) COMMITS -->
	{#if commitsAhead.length > 0}
		<div>
			{#each commitsAhead as commit, index}
				<CommitCard
					{commit}
					first={index === 0}
					last={index === commitsAhead.length - 1}
					isUnapplied={true}
					commitUrl={$gitHost?.commitUrl(commit.id)}
					type="local"
				>
					{#snippet lines(topHeightPx)}
						<LineGroup lineGroup={lineManager.get(commit.id)} {topHeightPx} />
					{/snippet}
				</CommitCard>
			{/each}
		</div>

		<CommitAction backgroundColor={false}>
			{#snippet lines()}
				<LineGroup lineGroup={lineManager.get(LineSpacer.Local)} topHeightPx={0} />
			{/snippet}
			{#snippet action()}
				<div class="local-actions-wrapper">
					<Button
						wide
						style={resetBaseTo.local.color}
						icon={onlyLocalAhead ? undefined : 'warning'}
						kind="solid"
						tooltip={pushButtonTooltip}
						loading={baseBranchIsUpdating || confirmResetModalOpen}
						disabled={$mode?.type !== 'OpenWorkspace' ||
							baseBranchIsUpdating ||
							confirmResetModalOpen ||
							baseBranchConflicted}
						onclick={resetBaseTo.local.handler}
					>
						{onlyLocalAhead ? 'Push' : resetBaseTo.local.title}
					</Button>

					{#if onlyLocalAhead}
						<Button
							wide
							style="error"
							icon="warning"
							kind="solid"
							tooltip="Discard your local changes"
							disabled={$mode?.type !== 'OpenWorkspace' || integrateUpstreamModalOpen}
							loading={integrateUpstreamModalOpen}
							onclick={handleMergeUpstream}
						>
							Discard local changes
						</Button>
					{/if}
				</div>
			{/snippet}
		</CommitAction>
	{/if}

	<!-- LOCAL AND REMOTE COMMITS -->
	<div>
		{#each localAndRemoteCommits as commit, index}
			<CommitCard
				{commit}
				first={index === 0}
				last={index === localAndRemoteCommits.length - 1}
				isUnapplied={true}
				commitUrl={$gitHost?.commitUrl(commit.id)}
				type="localAndRemote"
			>
				{#snippet lines(topHeightPx)}
					<LineGroup lineGroup={lineManager.get(commit.id)} {topHeightPx} />
				{/snippet}
			</CommitCard>
		{/each}
	</div>
</div>

<Modal
	width="small"
	bind:this={updateTargetModal}
	title="Merge Upstream Work"
	onSubmit={(close) => {
		updateBaseBranch();
		if (mergeUpstreamWarningDismissedCheckbox) {
			mergeUpstreamWarningDismissed.set(true);
		}
		close();
	}}
>
	<div class="modal-content">
		<p class="text-14 text-body">You are about to merge upstream work from your base branch.</p>
	</div>
	<div class="modal-content">
		<h4 class="text-14 text-body text-semibold">What will this do?</h4>
		<p class="modal__small-text text-12 text-body">
			We will try to merge the work that is upstream into each of your virtual branches, so that
			they are all up to date.
		</p>
		<p class="modal__small-text text-12 text-body">
			Any virtual branches that we can't merge cleanly, we will unapply and mark with a blue dot.
			You can merge these manually later.
		</p>
		<p class="modal__small-text text-12 text-body">
			Any virtual branches that are fully integrated upstream will be automatically removed.
		</p>
	</div>

	<label class="modal__dont-show-again" for="dont-show-again">
		<Checkbox name="dont-show-again" bind:checked={mergeUpstreamWarningDismissedCheckbox} />
		<span class="text-12">Don't show this again</span>
	</label>

	{#snippet controls(close)}
		<Button style="ghost" outline onclick={close}>Cancel</Button>
		<Button style="pop" kind="solid" type="submit">Merge Upstream</Button>
	{/snippet}
</Modal>

{#if resetBaseStrategy}
	<Modal
		width="small"
		title={resetBaseTo[resetBaseStrategy].title}
		bind:this={confirmResetModal}
		onSubmit={async (close) => {
			if (resetBaseStrategy) await resetBaseTo[resetBaseStrategy].action();
			close();
		}}
	>
		<div class="modal-content">
			<p class="text-12">
				{resetBaseTo[resetBaseStrategy].content}
				<br />
				<br />
				{#if resetBaseStrategy === 'local'}
					{base.divergedBehind.length > 1
						? `The ${base.divergedBehind.length} commits in the remote branch will be lost.`
						: 'The commit in the remote branch will be lost.'}
				{:else if resetBaseStrategy === 'remote'}
					{base.divergedAhead.length > 1
						? `The ${base.divergedAhead.length} commits in the local branch will be lost.`
						: 'The commit in the local branch will be lost.'}
				{/if}
			</p>
		</div>

		{#snippet controls(close)}
			<Button style="ghost" outline onclick={close}>Cancel</Button>
			<Button style="error" kind="solid" type="submit" icon="warning"
				>{resetBaseTo[resetBaseStrategy!].title}</Button
			>
		{/snippet}
	</Modal>
{/if}

<IntegrateUpstreamModal bind:this={integrateUpstreamModal} />

<style>
	.header-wrapper {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.message-wrapper {
		display: flex;
		flex-direction: column;
		margin-bottom: 20px;
		gap: 16px;
	}
	.wrapper {
		display: flex;
		flex-direction: column;
	}

	.info-text {
		opacity: 0.5;
	}

	.local-actions-wrapper {
		display: flex;
		width: 100%;
		gap: 8px;
	}

	.modal-content {
		display: flex;
		flex-direction: column;
		gap: 10px;
		margin-bottom: 20px;

		&:last-child {
			margin-bottom: 0;
		}
	}

	.modal__small-text {
		opacity: 0.6;
	}

	.modal__dont-show-again {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 14px;
		background-color: var(--clr-bg-2);
		border-radius: var(--radius-m);
		margin-bottom: 6px;
	}
</style>
