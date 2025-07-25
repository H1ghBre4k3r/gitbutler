<script lang="ts">
	import { SelectedOwnership } from '$lib/branches/ownership';
	import { maybeGetContextStore } from '@gitbutler/shared/context';
	import Badge from '@gitbutler/ui/Badge.svelte';
	import Checkbox from '@gitbutler/ui/Checkbox.svelte';
	import type { ConflictEntries } from '$lib/files/conflicts';
	import type { AnyFile } from '$lib/files/file';
	import type { Writable } from 'svelte/store';

	interface Props {
		title: string;
		files: AnyFile[];
		showCheckboxes?: boolean;
		conflictedFiles?: ConflictEntries;
	}

	const { title, files, showCheckboxes = false, conflictedFiles }: Props = $props();

	const selectedOwnership: Writable<SelectedOwnership> | undefined =
		maybeGetContextStore(SelectedOwnership);

	function selectAll(files: AnyFile[]) {
		if (!selectedOwnership) return;
		files.forEach((f) =>
			selectedOwnership.update((ownership) => ownership.select(f.id, ...f.hunks))
		);
	}

	function isAllChecked(selectedOwnership: SelectedOwnership | undefined): boolean {
		if (!selectedOwnership) return false;
		return files.every((f) => f.hunks.every((h) => selectedOwnership.isSelected(f.id, h.id)));
	}

	function isIndeterminate(selectedOwnership: SelectedOwnership | undefined): boolean {
		if (!selectedOwnership) return false;
		if (files.length <= 1) return false;

		let file = files[0] as AnyFile;
		let prev = selectedOwnership.isSelected(file.id, ...file.hunkIds);
		for (let i = 1; i < files.length; i++) {
			file = files[i] as AnyFile;
			const contained = selectedOwnership.isSelected(file.id, ...file.hunkIds);
			if (contained !== prev) {
				return true;
			}
		}
		return false;
	}

	const indeterminate = $derived(selectedOwnership ? isIndeterminate($selectedOwnership) : false);
	const checked = $derived(isAllChecked($selectedOwnership));
</script>

<div class="header">
	<div class="header__left">
		{#if showCheckboxes && files.length > 1}
			<Checkbox
				small
				{checked}
				{indeterminate}
				onchange={(e: Event & { currentTarget: EventTarget & HTMLInputElement }) => {
					const isChecked = e.currentTarget.checked;
					if (isChecked) {
						selectAll(files);
					} else {
						selectedOwnership?.update((ownership) => ownership.clearSelection());
					}
				}}
			/>
		{/if}
		<div class="header__title text-13 text-semibold">
			<span>{title}</span>
			<Badge>{files.length + (conflictedFiles?.entries.size || 0)}</Badge>
		</div>
	</div>
</div>

<style lang="postcss">
	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px;
		border-bottom: none;
		border-radius: var(--radius-m) var(--radius-m) 0 0;
		background-color: transparent !important;
	}
	.header__title {
		display: flex;
		align-items: center;
		gap: 4px;
		color: var(--clr-scale-ntrl-0);
	}
	.header__left {
		display: flex;
		gap: 10px;
	}
</style>
