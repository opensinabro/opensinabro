import { get } from "./fetch";
import type { Fetched } from "./fetch";

export type BlockEntry = {
  target: string;
  group: string;
  reason: string;
  createdAt: string;
  removedAt: string | null;
};

export type PermissionEntry = {
  userName: string;
  permission: string;
  grantedAt: string;
  revokedAt: string | null;
};

export type BlockHistoryView = {
  blocks: BlockEntry[];
  permissions: PermissionEntry[];
};

export function fetchBlockHistory(): Promise<Fetched<BlockHistoryView>> {
  return get<BlockHistoryView>("/api/block-history");
}

export type ContributionEntry = {
  title: string;
  sequence: number;
  createdAt: string;
  comment: string;
};

export type ContributionsView = {
  name: string;
  entries: ContributionEntry[];
};

export function fetchContributions(
  name: string,
): Promise<Fetched<ContributionsView>> {
  return get<ContributionsView>(
    `/api/users/${encodeURIComponent(name)}/contributions`,
  );
}

export function fetchVerification(
  token: string,
): Promise<Fetched<{ verified: boolean }>> {
  return get<{ verified: boolean }>(
    `/api/verify?token=${encodeURIComponent(token)}`,
  );
}
