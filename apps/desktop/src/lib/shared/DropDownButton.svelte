<script lang="ts">
	import ContextMenu from '$lib/components/contextmenu/ContextMenu.svelte';
	import Button from '@gitbutler/ui/Button.svelte';
	import Tooltip from '@gitbutler/ui/Tooltip.svelte';
	import type iconsJson from '@gitbutler/ui/data/icons.json';
	import type { ComponentColor, ComponentStyleKind } from '@gitbutler/ui/utils/colorTypes';
	import type { Snippet } from 'svelte';

	interface DropDownButtonProps {
		icon?: keyof typeof iconsJson;
		style?: ComponentColor;
		kind?: ComponentStyleKind;
		outline?: boolean;
		disabled?: boolean;
		loading?: boolean;
		wide?: boolean;
		tooltip?: string;
		menuPosition?: 'top' | 'bottom';
		children: Snippet;
		contextMenuSlot: Snippet;
		onclick?: (e: MouseEvent) => void;
	}

	const {
		icon,
		style = 'neutral',
		kind = 'soft',
		outline = false,
		disabled = false,
		loading = false,
		wide = false,
		tooltip,
		menuPosition = 'bottom',
		children,
		contextMenuSlot,
		onclick
	}: DropDownButtonProps = $props();

	let contextMenu = $state<ReturnType<typeof ContextMenu>>();
	let iconEl = $state<HTMLElement>();
	let visible = $state(false);

	export function show() {
		visible = true;
		contextMenu?.open();
	}

	export function close() {
		visible = false;
		contextMenu?.close();
	}
</script>

<Tooltip text={tooltip}>
	<div class="dropdown-wrapper" class:wide>
		<div class="dropdown">
			<Button
				{style}
				{icon}
				{kind}
				{outline}
				reversedDirection
				disabled={disabled || loading}
				dropdownChild
				{onclick}
			>
				{@render children()}
			</Button>
			<Button
				bind:el={iconEl}
				{style}
				{kind}
				{outline}
				icon={visible ? 'chevron-up' : 'chevron-down'}
				{loading}
				disabled={disabled || loading}
				dropdownChild
				onclick={() => {
					visible = !visible;
					contextMenu?.toggle();
				}}
			/>
		</div>
		<ContextMenu
			bind:this={contextMenu}
			target={iconEl}
			verticalAlign={menuPosition}
			onclose={() => {
				visible = false;
			}}
		>
			{@render contextMenuSlot()}
		</ContextMenu>
	</div>
</Tooltip>

<style lang="postcss">
	.dropdown-wrapper {
		/* display set directly on element */
		position: relative;
	}

	.dropdown {
		display: flex;
		flex-grow: 1;
		align-items: center;
	}
	.wide {
		width: 100%;
	}
</style>
