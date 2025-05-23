import { apiToChatMessage, type ApiChatMessage, type ChatMessage } from '$lib/chat/types';
import { apiToPatch, type ApiPatchCommit, type PatchCommit } from '$lib/patches/types';
import {
	apiToUserSimple,
	isApiUserSimple,
	type ApiUserSimple,
	type UserSimple
} from '$lib/users/types';
import type { LoadableData } from '$lib/network/types';
import type { BrandedId } from '$lib/utils/branding';

type ApiPatchEventBase = {
	uuid: string;
	user: ApiUserSimple | null;
	event_type: string;
	data: unknown;
	object: unknown;
	created_at: string;
	updated_at: string;
};

function isApiPatchEventBase(data: unknown): data is ApiPatchEventBase {
	return (
		typeof data === 'object' &&
		data !== null &&
		typeof (data as any).uuid === 'string' &&
		typeof (data as any).event_type === 'string' &&
		(isApiUserSimple((data as any).user) || (data as any).user === null)
	);
}

export type ApiChatEvent = ApiPatchEventBase & {
	event_type: 'chat';
	object: ApiChatMessage | null;
};

export function isApiChatEvent(data: unknown): data is ApiChatEvent {
	return isApiPatchEventBase(data) && (data as any).event_type === 'chat';
}

export type ApiChatReactionEvent = ApiPatchEventBase & {
	event_type: 'chat_reaction';
	object: ApiChatMessage;
	data: { reaction: string };
};

export function isApiChatReactionEvent(data: unknown): data is ApiChatReactionEvent {
	return (
		isApiPatchEventBase(data) &&
		(data as any).event_type === 'chat_reaction' &&
		typeof (data as any).data === 'object' &&
		(data as any).data !== null &&
		typeof (data as any).data.reaction === 'string'
	);
}

export type ApiChatReplyEvent = ApiPatchEventBase & {
	event_type: 'chat_reply';
	object: ApiChatMessage;
	data: { in_reply_to: string };
};

export function isApiChatReplyEvent(data: unknown): data is ApiChatReplyEvent {
	return (
		isApiPatchEventBase(data) &&
		(data as any).event_type === 'chat_reply' &&
		typeof (data as any).data === 'object' &&
		(data as any).data !== null &&
		typeof (data as any).data.in_reply_to === 'string'
	);
}

export type ApiPatchVersionEvent = ApiPatchEventBase & {
	event_type: 'patch_version';
	object: ApiPatchCommit;
};

export type ApiPatchStatusEvent = ApiPatchEventBase & {
	data: { status: boolean; message: string | null };
	event_type: 'patch_status';
	object: ApiPatchCommit;
};

export function isApiPatchVersionEvent(data: unknown): data is ApiPatchVersionEvent {
	return isApiPatchEventBase(data) && (data as any).event_type === 'patch_version';
}

export function isApiPatchStatusEvent(data: unknown): data is ApiPatchStatusEvent {
	return isApiPatchEventBase(data) && (data as any).event_type === 'patch_status';
}

export type ApiIssueUpdate = {
	uuid: string;
	user: ApiUserSimple | null;
	outdated: boolean;
	issue: boolean;
	resolved: boolean;
	thread_id: string | null;
	comment: string;
	diffSha: string | null;
	range: string | null;
	diff_path: string | null;
	diff_patch_array: string[] | null;
	created_at: string;
	updated_at: string;
};

export type ApiIssueUpdateEvent = ApiPatchEventBase & {
	event_type: 'issue_status';
	object: ApiIssueUpdate;
};

export function isApiIssueUpdateEvent(data: unknown): data is ApiIssueUpdateEvent {
	return isApiPatchEventBase(data) && (data as any).event_type === 'issue_status';
}

export type ApiPatchEvent =
	| ApiChatEvent
	| ApiChatReactionEvent
	| ApiChatReplyEvent
	| ApiPatchVersionEvent
	| ApiPatchStatusEvent
	| ApiIssueUpdateEvent;

export function isApiPatchEvent(data: unknown): data is ApiPatchEvent {
	return (
		isApiChatEvent(data) ||
		isApiChatReactionEvent(data) ||
		isApiChatReplyEvent(data) ||
		isApiPatchVersionEvent(data) ||
		isApiPatchStatusEvent(data) ||
		isApiIssueUpdateEvent(data)
	);
}

type PatchEventBase = {
	uuid: string;
	user: UserSimple | undefined;
	eventType: string;
	data: unknown;
	object: unknown;
	createdAt: string;
	updatedAt: string;
};

export type ChatEvent = PatchEventBase & {
	eventType: 'chat';
	object: ChatMessage;
};

export type ChatReactionEvent = PatchEventBase & {
	eventType: 'chat_reaction';
	object: ChatMessage;
	data: { reaction: string };
};

export type ChatReplyEvent = PatchEventBase & {
	eventType: 'chat_reply';
	object: ChatMessage;
	data: { inReplyTo: string };
};

export type PatchVersionEvent = PatchEventBase & {
	eventType: 'patch_version';
	object: PatchCommit;
};

export type PatchStatusEvent = PatchEventBase & {
	data: { status: boolean; message: string | undefined };
	eventType: 'patch_status';
	object: PatchCommit;
};

export function apiToPatchStatusData(api: ApiPatchStatusEvent['data']): PatchStatusEvent['data'] {
	return {
		status: api.status,
		message: api.message ?? undefined
	};
}

export type IssueUpdate = {
	uuid: string;
	user: UserSimple | undefined;
	outdated: boolean;
	issue: boolean;
	resolved: boolean;
	threadId: string | undefined;
	comment: string;
	diffSha: string | undefined;
	range: string | undefined;
	diffPath: string | undefined;
	diffPatchArray: string[] | undefined;
	createdAt: string;
	updatedAt: string;
};

export function apiToIssueUpdate(api: ApiIssueUpdate): IssueUpdate {
	return {
		uuid: api.uuid,
		user: api.user ? apiToUserSimple(api.user) : undefined,
		outdated: api.outdated,
		issue: api.issue,
		resolved: api.resolved,
		threadId: api.thread_id ?? undefined,
		comment: api.comment,
		diffSha: api.diffSha ?? undefined,
		range: api.range ?? undefined,
		diffPath: api.diff_path ?? undefined,
		diffPatchArray: api.diff_patch_array ?? undefined,
		createdAt: api.created_at,
		updatedAt: api.updated_at
	};
}

export type IssueUpdateEvent = PatchEventBase & {
	eventType: 'issue_status';
	object: IssueUpdate;
};

export type PatchEvent =
	| ChatEvent
	| ChatReactionEvent
	| ChatReplyEvent
	| PatchVersionEvent
	| PatchStatusEvent
	| IssueUpdateEvent;

export function apiToPatchEvent(api: ApiPatchEvent): PatchEvent | null {
	switch (api.event_type) {
		case 'chat':
			if (!api.object) return null;
			return {
				eventType: api.event_type,
				uuid: api.uuid,
				user: api.user ? apiToUserSimple(api.user) : undefined,
				data: api.data,
				object: apiToChatMessage(api.object),
				createdAt: api.created_at,
				updatedAt: api.updated_at
			};
		case 'chat_reaction':
			return {
				eventType: api.event_type,
				uuid: api.uuid,
				user: api.user ? apiToUserSimple(api.user) : undefined,
				data: { reaction: api.data.reaction },
				object: apiToChatMessage(api.object),
				createdAt: api.created_at,
				updatedAt: api.updated_at
			};
		case 'chat_reply':
			return {
				eventType: api.event_type,
				uuid: api.uuid,
				user: api.user ? apiToUserSimple(api.user) : undefined,
				data: { inReplyTo: api.data.in_reply_to },
				object: apiToChatMessage(api.object),
				createdAt: api.created_at,
				updatedAt: api.updated_at
			};
		case 'patch_version':
			// Ignore version 1 patches
			if (api.object.version === 1) return null;
			return {
				eventType: api.event_type,
				uuid: api.uuid,
				user: api.user ? apiToUserSimple(api.user) : undefined,
				data: api.data,
				object: apiToPatch(api.object),
				createdAt: api.created_at,
				updatedAt: api.updated_at
			};
		case 'patch_status':
			return {
				eventType: api.event_type,
				uuid: api.uuid,
				user: api.user ? apiToUserSimple(api.user) : undefined,
				data: apiToPatchStatusData(api.data),
				object: apiToPatch(api.object),
				createdAt: api.created_at,
				updatedAt: api.updated_at
			};
		case 'issue_status':
			return {
				eventType: api.event_type,
				uuid: api.uuid,
				user: api.user ? apiToUserSimple(api.user) : undefined,
				data: api.data,
				object: apiToIssueUpdate(api.object),
				createdAt: api.created_at,
				updatedAt: api.updated_at
			};
	}
}

type PatchEventChannelId = BrandedId<'PatchEventChannelId'>;

export type PatchEventChannel = {
	/**
	 * The unique identifier of the patch event channel.
	 *
	 * Built from the project ID and the change ID.
	 */
	id: PatchEventChannelId;
	projectId: string;
	changeId: string;
	events: PatchEvent[];
};

export function createPatchEventChannelKey(
	projectId: string,
	changeId: string
): PatchEventChannelId {
	return `${projectId}:${changeId}` as PatchEventChannelId;
}

export type LoadablePatchEventChannel = LoadableData<PatchEventChannel, PatchEventChannel['id']>;
