<script lang="ts">
	import { getColorFromBranchType } from '@gitbutler/ui/utils/getColorFromBranchType';
	interface Props {
		isBottomBranch?: boolean;
	}

	const { isBottomBranch }: Props = $props();
</script>

<div class="empty-series" style:--commit-color={getColorFromBranchType('LocalOnly')}>
	<div class="commit-line" class:dashed={isBottomBranch}></div>
	<div class="text-13 text-body empty-series__label">
		This is an empty branch.
		<br />
		Create or drag & drop commits here
	</div>
</div>

<style lang="postcss">
	.empty-series {
		display: flex;
	}
	.empty-series__label {
		width: 100%;
		padding: 20px 28px 26px 46px;
		color: var(--clr-text-2);
		opacity: 0.6;
	}
	.commit-line {
		position: absolute;
		left: 20px;
	}
</style>
