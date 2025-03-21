<script lang="ts">
	import AddSeriesModal from './AddSeriesModal.svelte';
	import BranchLabel from './BranchLabel.svelte';
	import BranchStatus from './BranchStatus.svelte';
	import Dropzones from './Dropzones.svelte';
	import SeriesDescription from './SeriesDescription.svelte';
	import SeriesHeaderStatusIcon from './SeriesHeaderStatusIcon.svelte';
	import BranchReview from '$components/BranchReview.svelte';
	import PullRequestCard from '$components/PullRequestCard.svelte';
	import ReviewDetailsModal from '$components/ReviewDetailsModal.svelte';
	import SeriesHeaderContextMenu from '$components/SeriesHeaderContextMenu.svelte';
	import { PromptService } from '$lib/ai/promptService';
	import { AIService } from '$lib/ai/service';
	import { BaseBranch } from '$lib/baseBranch/baseBranch';
	import { BranchStack } from '$lib/branches/branch';
	import { PatchSeries } from '$lib/branches/branch';
	import { BranchController } from '$lib/branches/branchController';
	import {
		allPreviousSeriesHavePrNumber,
		childBranch,
		parentBranch
	} from '$lib/branches/virtualBranchService';
	import { type CommitStatus } from '$lib/commits/commit';
	import { projectAiGenEnabled } from '$lib/config/config';
	import { FileService } from '$lib/files/fileService';
	import { closedStateSync } from '$lib/forge/closedStateSync.svelte';
	import { getForge } from '$lib/forge/interface/forge';
	import { getForgeListingService } from '$lib/forge/interface/forgeListingService';
	import { getForgePrService } from '$lib/forge/interface/forgePrService';
	import { ProjectService } from '$lib/project/projectService';
	import { openExternalUrl } from '$lib/utils/url';
	import { getContext, getContextStore } from '@gitbutler/shared/context';
	import { reactive } from '@gitbutler/shared/reactiveUtils.svelte';
	import Button from '@gitbutler/ui/Button.svelte';
	import ContextMenu from '@gitbutler/ui/ContextMenu.svelte';
	import Modal from '@gitbutler/ui/Modal.svelte';
	import PopoverActionsContainer from '@gitbutler/ui/popoverActions/PopoverActionsContainer.svelte';
	import PopoverActionsItem from '@gitbutler/ui/popoverActions/PopoverActionsItem.svelte';
	import { getColorFromBranchType } from '@gitbutler/ui/utils/getColorFromBranchType';
	import { tick } from 'svelte';

	interface Props {
		branch: PatchSeries;
		isTopBranch: boolean;
		lastPush: Date | undefined;
	}

	const { branch, isTopBranch, lastPush }: Props = $props();

	let descriptionVisible = $state(!!branch.description);

	const aiService = getContext(AIService);
	const promptService = getContext(PromptService);
	const fileService = getContext(FileService);
	const stackStore = getContextStore(BranchStack);
	const projectService = getContext(ProjectService);
	const stack = $derived($stackStore);
	const project = projectService.project;

	const parent = $derived(
		parentBranch(
			branch,
			stack.validSeries.filter((b) => b.archived)
		)
	);
	const child = $derived(
		childBranch(
			branch,
			stack.validSeries.filter((b) => !b.archived)
		)
	);

	const aiGenEnabled = $derived(!!$project && projectAiGenEnabled($project.id));
	const branchController = getContext(BranchController);
	const baseBranch = getContextStore(BaseBranch);
	const prService = getForgePrService();
	const forge = getForge();

	const upstreamName = $derived(branch.upstreamReference ? branch.name : undefined);
	const forgeBranch = $derived(upstreamName ? $forge?.branch(upstreamName) : undefined);
	const previousSeriesHavePrNumber = $derived(
		allPreviousSeriesHavePrNumber(branch.name, stack.validSeries)
	);

	let stackingAddSeriesModal = $state<ReturnType<typeof AddSeriesModal>>();
	let prDetailsModal = $state<ReturnType<typeof ReviewDetailsModal>>();
	let kebabContextMenu = $state<ReturnType<typeof ContextMenu>>();
	let stackingContextMenu = $state<ReturnType<typeof SeriesHeaderContextMenu>>();
	let confirmCreatePrModal = $state<ReturnType<typeof Modal>>();
	let kebabContextMenuTrigger = $state<HTMLButtonElement>();
	let seriesHeaderEl = $state<HTMLDivElement>();
	let seriesDescriptionEl = $state<HTMLTextAreaElement>();
	let contextMenuOpened = $state(false);

	const topPatch = $derived(branch?.patches[0]);
	const branchType = $derived<CommitStatus>(topPatch?.status ?? 'LocalOnly');
	const lineColor = $derived(getColorFromBranchType(branchType));
	const hasNoCommits = $derived(branch.upstreamPatches.length === 0 && branch.patches.length === 0);
	const parentIsPushed = $derived(!!parent?.upstreamReference);
	const parentIsIntegrated = $derived(!!parent?.integrated);
	const hasParent = $derived(!!parent);
	const isPushed = $derived(!!branch.upstreamReference);

	// Pretty cumbersome way of getting the PR number, would be great if we can
	// make it more concise somehow.
	const forgeListing = getForgeListingService();
	const prStore = $derived($forgeListing?.prs);
	const prs = $derived(prStore ? $prStore : undefined);

	const listedPr = $derived(prs?.find((pr) => pr.sourceBranch === upstreamName));
	const prNumber = $derived(branch.prNumber || listedPr?.number);

	const prMonitor = $derived(prNumber ? $prService?.prMonitor(prNumber) : undefined);
	const pr = $derived(prMonitor?.pr);
	const sourceBranch = $derived($pr?.sourceBranch); // Deduplication.
	const mergedIncorrectly = $derived(prMonitor?.mergedIncorrectly);

	// Do not create a checks monitor if pull request is merged or from a fork.
	// For more information about unavailability of check-runs for forked repos,
	// see GitHub docs at:
	// https://docs.github.com/en/rest/checks/runs?apiVersion=2022-11-28#list-check-runs-in-a-check-suite
	// TODO: Make this forge specific by moving it into ForgePrMonitor.
	const shouldCheck = $derived($pr && !$pr.fork && !$pr.merged); // Deduplication.
	const checksMonitor = $derived(
		sourceBranch && shouldCheck ? $forge?.checksMonitor(sourceBranch) : undefined
	);

	// Extra reference to avoid potential infinite loop.
	let lastSeenPush: Date | undefined;

	// Without lastSeenPush this code has gone into an infinite loop, where lastPush
	// seemingly kept updating as a result of calling updateStatusAndChecks.
	// TODO: Refactor such that we do not need `$effect`.
	$effect(() => {
		if (!lastPush) return;
		if (!lastSeenPush || lastPush > lastSeenPush) {
			updateStatusAndChecks();
		}
		lastSeenPush = lastPush;
	});

	async function handleReloadPR() {
		await updateStatusAndChecks();
	}

	async function updateStatusAndChecks() {
		await Promise.allSettled([prMonitor?.refresh(), checksMonitor?.update()]);
	}

	/**
	 * We are starting to store pull request id's locally so if we find one that does not have
	 * one locally stored then we set it once.
	 *
	 * TODO: Remove this after transition is complete.
	 */
	$effect(() => {
		if (
			$forge?.name === 'github' &&
			!branch.prNumber &&
			listedPr?.number &&
			listedPr.number !== branch.prNumber
		) {
			branchController.updateBranchPrNumber(stack.id, branch.name, listedPr.number);
		}
	});

	function confirmCreatePR(close: () => void) {
		close();
		prDetailsModal?.show();
	}

	function handleOpenBranchReview() {
		if (!previousSeriesHavePrNumber) {
			confirmCreatePrModal?.show();
			return;
		}
		prDetailsModal?.show();
	}

	async function handleReopenPr() {
		if (!$pr) {
			return;
		}
		await $prService?.reopen($pr?.number);
		await $forgeListing?.refresh();
		await handleReloadPR();
	}

	function editTitle(title: string) {
		if (branch?.name && title !== branch.name) {
			branchController.updateSeriesName(stack.id, branch.name, title);
		}
	}

	async function editDescription(description: string | undefined | null) {
		if (description) {
			await branchController.updateSeriesDescription(stack.id, branch.name, description);
		}
	}

	async function toggleDescription() {
		descriptionVisible = !descriptionVisible;

		if (!descriptionVisible) {
			await branchController.updateSeriesDescription(stack.id, branch.name, '');
		} else {
			await tick();
			seriesDescriptionEl?.focus();
		}
	}

	async function generateBranchName() {
		if (!aiGenEnabled || !branch || !$project) return;

		let hunk_promises = branch.patches.flatMap(async (p) => {
			let files = await fileService.listCommitFiles($project.id, p.id);
			return files.flatMap((f) =>
				f.hunks.map((h) => {
					return { filePath: f.path, diff: h.diff };
				})
			);
		});
		let hunks = (await Promise.all(hunk_promises)).flat();

		const prompt = promptService.selectedBranchPrompt($project.id);
		const message = await aiService.summarizeBranch({
			hunks,
			branchTemplate: prompt
		});

		if (message && message !== branch.name) {
			branchController.updateSeriesName(stack.id, branch.name, message);
		}
	}

	async function onCreateNewPr() {
		// Make sure the listing result is up-to-date so that we don't
		// automatically set it back to what it was. If a branch has no
		// pr attached we look for any open prs with a matching branch
		// name, and save it to the branch.
		await $forgeListing?.refresh();

		if (!branch.prNumber) {
			throw new Error('Failed to discard pr, try reloading the app.');
		}

		// Delete the reference stored on disk.
		branchController.updateBranchPrNumber(stack.id, branch.name, null);
		kebabContextMenu?.close();

		// Display create pr modal after a slight delay, this prevents
		// interference with the closing context menu. It also feels nice
		// that these two things are not happening at the same time.
		setTimeout(() => handleOpenBranchReview(), 250);
	}

	closedStateSync(reactive(() => branch));
</script>

<AddSeriesModal bind:this={stackingAddSeriesModal} parentSeriesName={branch.name} />

<SeriesHeaderContextMenu
	stackId={stack.id}
	bind:this={stackingContextMenu}
	bind:contextMenuEl={kebabContextMenu}
	leftClickTrigger={kebabContextMenuTrigger}
	rightClickTrigger={seriesHeaderEl}
	headName={branch.name}
	seriesCount={stack.validSeries?.length ?? 0}
	{isTopBranch}
	{toggleDescription}
	description={branch.description ?? ''}
	onGenerateBranchName={generateBranchName}
	onAddDependentSeries={() => stackingAddSeriesModal?.show()}
	onOpenInBrowser={() => {
		const url = forgeBranch?.url;
		if (url) openExternalUrl(url);
	}}
	hasForgeBranch={!!forgeBranch}
	pr={$pr}
	openPrDetailsModal={handleOpenBranchReview}
	{branchType}
	onMenuToggle={(isOpen, isLeftClick) => {
		if (isLeftClick) {
			contextMenuOpened = isOpen;
		}
	}}
	{parentIsPushed}
	{hasParent}
	{onCreateNewPr}
/>

<div
	role="article"
	class="branch-header"
	bind:this={seriesHeaderEl}
	oncontextmenu={(e) => {
		e.preventDefault();
		kebabContextMenu?.toggle(e);
	}}
>
	<Dropzones type="commit">
		<PopoverActionsContainer class="branch-actions-menu" stayOpen={contextMenuOpened}>
			{#if isTopBranch}
				<PopoverActionsItem
					icon="plus-small"
					tooltip="Add dependent branch"
					onclick={() => {
						stackingAddSeriesModal?.show();
					}}
				/>
			{/if}
			{#if forgeBranch}
				<PopoverActionsItem
					icon="open-link"
					tooltip="Open in browser"
					onclick={() => {
						const url = forgeBranch?.url;
						if (url) openExternalUrl(url);
					}}
				/>
			{/if}
			<PopoverActionsItem
				bind:el={kebabContextMenuTrigger}
				activated={contextMenuOpened}
				icon="kebab"
				tooltip="More options"
				onclick={() => {
					kebabContextMenu?.toggle();
				}}
			/>
		</PopoverActionsContainer>

		<div class="branch-info">
			<SeriesHeaderStatusIcon
				lineTop={isTopBranch ? false : true}
				icon={branchType === 'Integrated' ? 'tick-small' : 'branch-small'}
				iconColor="var(--clr-core-ntrl-100)"
				color={lineColor}
			/>
			<div class="branch-info__content">
				<div class="text-14 text-bold branch-info__name">
					{#if forgeBranch}
						<span class="remote-name">
							{$baseBranch.pushRemoteName ? `${$baseBranch.pushRemoteName} /` : 'origin /'}
						</span>
					{/if}
					<BranchLabel
						name={branch.name}
						onChange={(name) => editTitle(name)}
						readonly={!!forgeBranch}
						onDblClick={() => {
							if (branchType !== 'Integrated') {
								stackingContextMenu?.showSeriesRenameModal?.();
							}
						}}
					/>
				</div>
				{#if descriptionVisible}
					<div class="branch-info__description">
						<div class="branch-action__line" style:--bg-color={lineColor}></div>
						<SeriesDescription
							bind:textAreaEl={seriesDescriptionEl}
							value={branch.description ?? ''}
							onBlur={(value) => editDescription(value)}
							onEmpty={() => toggleDescription()}
						/>
					</div>
				{/if}
			</div>
		</div>
		{#if !hasNoCommits}
			<BranchReview {branch} openForgePullRequest={handleOpenBranchReview}>
				{#snippet branchLine()}
					<div class="branch-action__line" style:--bg-color={lineColor}></div>
				{/snippet}
				{#snippet pullRequestCard(pr)}
					<PullRequestCard
						reloadPR={handleReloadPR}
						reopenPr={handleReopenPr}
						openPrDetailsModal={handleOpenBranchReview}
						{pr}
						{checksMonitor}
						{prMonitor}
						{isPushed}
						{child}
						{hasParent}
						{parentIsPushed}
					/>
				{/snippet}
				{#snippet branchStatus()}
					<BranchStatus
						{mergedIncorrectly}
						{isPushed}
						{hasParent}
						{parentIsPushed}
						{parentIsIntegrated}
					/>
				{/snippet}
			</BranchReview>
		{/if}

		<ReviewDetailsModal bind:this={prDetailsModal} currentSeries={branch} stackId={stack.id} />

		<Modal
			width="small"
			type="warning"
			title="Create Pull Request"
			bind:this={confirmCreatePrModal}
			onSubmit={confirmCreatePR}
		>
			{#snippet children()}
				<p class="text-13 text-body helper-text">
					It's strongly recommended to create pull requests starting with the branch at the base of
					the stack.
					<br />
					Do you still want to create this pull request?
				</p>
			{/snippet}
			{#snippet controls(close)}
				<Button kind="outline" onclick={close}>Cancel</Button>
				<Button style="warning" type="submit">Create Pull Request</Button>
			{/snippet}
		</Modal>
	</Dropzones>
</div>

<style lang="postcss">
	.branch-header {
		position: relative;
		display: flex;
		align-items: center;
		flex-direction: column;

		&:not(:last-child) {
			border-bottom: 1px solid var(--clr-border-2);
		}

		&:hover,
		&:focus-within {
			& :global(.branch-actions-menu) {
				--show: true;
			}
		}
	}

	.branch-info {
		width: 100%;
		padding-right: 14px;
		display: flex;
		justify-content: flex-start;
		align-items: center;

		.remote-name {
			min-width: max-content;
			padding: 0 0 0 2px;
			color: var(--clr-scale-ntrl-60);
		}
	}

	.branch-info__name {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		min-width: 0;
		flex-grow: 1;
	}

	.branch-info__content {
		overflow: hidden;
		flex: 1;
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 14px 0;
		margin-left: -2px;
	}

	.branch-action__line {
		min-width: 2px;
		margin: 0 22px 0 20px;
		background-color: var(--bg-color, var(--clr-border-3));
	}
</style>
