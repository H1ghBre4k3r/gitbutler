import { type Reactive, reactive } from '@gitbutler/shared/storeUtils';
import {
	createEntityAdapter,
	createSlice,
	type EntityState,
	type ThunkDispatch,
	type UnknownAction
} from '@reduxjs/toolkit';

export class HunkSelection {
	private state = $state([]);
}

type HunkRange = {
	start: number;
	lines: number;
};

type HunkHeader = {
	oldStart: number;
	oldLines: number;
	newStart: number;
	newLines: number;
};

/**
 * Representation of visually selected hunk.
 */
type SelectedHunk = (
	| {
			type: 'full';
	  }
	| {
			type: 'partial';
			ranges: HunkRange;
	  }
) &
	HunkHeader;

/**
 * Representation of visually selected file.
 */
type SelectedFile =
	| {
			type: 'full';
			path: string;
	  }
	| {
			type: 'partial';
			path: string;
			hunks: SelectedHunk[];
	  };

export const changeSelectionAdapter = createEntityAdapter<SelectedFile, SelectedFile['path']>({
	selectId: (change) => change.path,
	sortComparer: (a, b) => a.path.localeCompare(b.path)
});

const { selectById, selectAll } = changeSelectionAdapter.getSelectors();

export const changeSelectionSlice = createSlice({
	name: 'changeSelection',
	initialState: changeSelectionAdapter.getInitialState(),
	reducers: {
		addOne: changeSelectionAdapter.addOne,
		removeOne: changeSelectionAdapter.removeOne,
		removeMany: changeSelectionAdapter.removeMany,
		removeAll: changeSelectionAdapter.removeAll,
		upsertOne: changeSelectionAdapter.upsertOne
	},
	selectors: { selectById, selectAll }
});

const { addOne, removeOne, removeMany, removeAll, upsertOne } = changeSelectionSlice.actions;

export class ChangeSelectionService {
	/** The change selection slice of the full redux state. */
	private state = $state<EntityState<SelectedFile, string>>(changeSelectionSlice.getInitialState());

	constructor(
		reactiveState: Reactive<typeof this.state>,
		private dispatch: ThunkDispatch<any, any, UnknownAction>
	) {
		$effect(() => {
			this.state = reactiveState.current;
		});
	}

	getById(path: string): Reactive<SelectedFile | undefined> {
		const selected = $derived(selectById(this.state, path));
		return reactive(() => selected);
	}

	addFull(path: string) {
		this.dispatch(addOne({ type: 'full', path }));
	}

	addPartial(path: string, hunks: SelectedHunk[]) {
		this.dispatch(addOne({ type: 'partial', path, hunks }));
	}

	update(path: string, hunks: SelectedHunk[]) {
		if (hunks.length === 0) {
			this.dispatch(upsertOne({ path, type: 'full' }));
		} else {
			this.dispatch(upsertOne({ path, type: 'partial', hunks }));
		}
	}

	remove(path: string) {
		this.dispatch(removeOne(path));
	}

	/** Clears any selected items that are not in `paths`.  */
	retain(paths: string[] | undefined) {
		if (paths === undefined) {
			this.dispatch(removeAll());
			return;
		}
		const selection = $derived(selectAll(this.state));
		const expired = [];
		for (const change of selection) {
			if (!paths.includes(change.path)) {
				expired.push(change.path);
			}
		}
		if (expired.length > 0) {
			this.dispatch(removeMany(expired));
		}
	}
}
