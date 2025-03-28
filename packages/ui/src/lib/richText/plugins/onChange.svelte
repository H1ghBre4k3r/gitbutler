<script lang="ts" module>
	export type OnChangeCallback = (
		value: string,
		textUpToAnchor: string | undefined,
		textAfterAnchor: string | undefined
	) => void;
</script>

<script lang="ts">
	import { getEditor } from '$lib/richText/context';
	import { getEditorTextAfterAnchor, getEditorTextUpToAnchor } from '$lib/richText/selection';
	import {
		$getRoot as getRoot,
		$getSelection as getSelection,
		$isRangeSelection as isRangeSelection
	} from 'lexical';

	type Props = {
		onChange?: OnChangeCallback;
	};

	const { onChange }: Props = $props();

	const editor = getEditor();

	let text = $state<string>();

	$effect(() => {
		return editor.registerUpdateListener(
			({ editorState, dirtyElements, dirtyLeaves, prevEditorState, tags }) => {
				if (
					tags.has('history-merge') ||
					(dirtyElements.size === 0 && dirtyLeaves.size === 0) ||
					prevEditorState.isEmpty()
				) {
					return;
				}

				editorState.read(() => {
					text = getRoot().getTextContent();
					const selection = getSelection();
					if (!isRangeSelection(selection)) {
						return;
					}

					const textUpToAnchor = getEditorTextUpToAnchor(selection);
					const textAfterAnchor = getEditorTextAfterAnchor(selection);
					onChange?.(text, textUpToAnchor, textAfterAnchor);
				});
			}
		);
	});

	export function getText(): string | undefined {
		return text;
	}
</script>
