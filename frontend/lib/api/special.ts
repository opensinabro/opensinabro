import { encodeTitle } from "@/lib/wiki-path";
import { get } from "./fetch";
import type { TitleEntry } from "./types";

type TitleListView = { entries: TitleEntry[] };

export function fetchNeededPages() {
  return get<TitleListView>("/api/needed-pages");
}

export function fetchOrphanedPages() {
  return get<TitleListView>("/api/orphaned-pages");
}

export function fetchUncategorizedPages() {
  return get<TitleListView>("/api/uncategorized-pages");
}

export function fetchOldPages() {
  return get<TitleListView>("/api/old-pages");
}

export function fetchPagesByLength(order: "shortest" | "longest") {
  return get<TitleListView>(`/api/pages-by-length?order=${order}`);
}

export function fetchStarred() {
  return get<TitleListView>("/api/starred");
}

export type NotificationItem = {
  kind: string;
  kindLabel: string;
  document: string;
  read: boolean;
  createdAt: string;
};

export function fetchNotifications() {
  return get<{ items: NotificationItem[] }>("/api/notifications");
}

export type SearchView = {
  query: string;
  redirect: string | null;
  results: TitleEntry[];
};

export function fetchSearch(query: string) {
  return get<SearchView>(`/api/search?q=${encodeURIComponent(query)}`);
}

export function fetchRandom() {
  return get<{ title: string | null }>("/api/random");
}

export function fetchLicense() {
  return get<{ engineNotice: string; contentLicense: string }>("/api/license");
}

export function fetchCategoryMembers(title: string) {
  return get<{ members: TitleEntry[] }>(`/api/category/${encodeTitle(title)}`);
}
