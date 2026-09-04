import { create } from "zustand";
import { searchWorkspace } from "../lib/tauriApi";
import type { SearchResult } from "../lib/types";

const DEBOUNCE_MS = 200;

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
/** Guards against an older, slower query's results overwriting a newer one. */
let latestQueryToken = 0;

interface SearchState {
  isOpen: boolean;
  query: string;
  results: SearchResult[];
  loading: boolean;

  open: () => void;
  close: () => void;
  setQuery: (query: string) => void;
}

export const useSearchStore = create<SearchState>((set) => ({
  isOpen: false,
  query: "",
  results: [],
  loading: false,

  open: () => set({ isOpen: true }),
  close: () => {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    set({ isOpen: false, query: "", results: [], loading: false });
  },

  setQuery: (query) => {
    set({ query });
    if (debounceTimer) clearTimeout(debounceTimer);

    if (!query.trim()) {
      set({ results: [], loading: false });
      return;
    }

    set({ loading: true });
    const token = ++latestQueryToken;
    debounceTimer = setTimeout(async () => {
      try {
        const results = await searchWorkspace(query);
        if (token === latestQueryToken) {
          set({ results, loading: false });
        }
      } catch {
        if (token === latestQueryToken) {
          set({ results: [], loading: false });
        }
      }
    }, DEBOUNCE_MS);
  },
}));
