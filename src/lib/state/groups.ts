import { AbletonColor } from '$lib/utils/colors';
import { get } from 'svelte/store';
import { appState, AppState, AudioFileItem } from './state.svelte';
import { loggingState, logger } from './logging';

export interface GroupsState {
  defs: Record<string, GroupDef>; // serialized definitions
  folders?: Record<string, string[]>; // optional UI “grouping groups” (just organization)
  _version?: number;
}

export type GroupDef =
  | { kind: 'op'; op: 'and' | 'or'; refs: string[] }
  | { kind: 'not'; ref: string }
  | { kind: 'query'; query: ItemQuery };

export type ItemQuery =
  | {
      kind: 'sectionPercent';
      sectionIndex: number;
      percent: number;
      orderBy?: 'index' | 'duration';
      take?: 'first' | 'last';
    }
  | { kind: 'lastOfEachSection' }
  | { kind: 'lastOfAllSections' }
  | { kind: 'where'; clause: WhereClause };

export type WhereClause =
  | { field: 'active'; eq: boolean }
  | { field: 'color'; eq: AbletonColor }
  | { field: 'duration'; gt?: number; lt?: number }
  | { field: 'path'; includes: string };

export type GroupResult = Set<string>;
export type GroupSelector = (state: AppState) => GroupResult;

export class GroupRegistry {
  private compiled = new Map<string, GroupSelector>();
  private cache = new Map<string, { version: number; value: GroupResult }>();

  constructor(private getDefs: () => Record<string, GroupDef> | undefined) {}

  /** Call whenever appState changes materially (or tie it to your existing _version bump) */
  eval(name: string, state: AppState): GroupResult {
    const version = state._version ?? 0;
    const isLogging = get(loggingState).groupsLog;

    const cached = this.cache.get(name);
    if (cached && cached.version === version) {
      if (isLogging) {
        logger.groups.cache(`Using cached result for "${name}" (version ${version}), size: ${cached.value.size}`);
      }
      return cached.value;
    }

    if (isLogging) {
      logger.groups.eval(`Evaluating group "${name}" (version ${version})`);
    }

    const selector = this.getOrCompile(name);
    const value = selector(state);

    this.cache.set(name, { version, value });
    
    if (isLogging) {
      logger.groups.success(`Evaluated group "${name}" -> ${value.size} items`, Array.from(value));
    }

    return value;
  }

  invalidateAll() {
    const isLogging = get(loggingState).groupsLog;
    const cacheSize = this.cache.size;
    
    this.cache.clear();
    
    if (isLogging) {
      logger.groups.cache(`Invalidated all cached groups (${cacheSize} entries cleared)`);
    }
  }

  private getOrCompile(name: string): GroupSelector {
    const existing = this.compiled.get(name);
    if (existing) return existing;

    const isLogging = get(loggingState).groupsLog;
    const defs = this.getDefs();
    const def = defs?.[name];
    
    if (!def) {
      if (isLogging) {
        logger.groups.error(`Unknown group "${name}"`);
      }
      throw new Error(`Unknown group "${name}"`);
    }

    if (isLogging) {
      logger.groups.info(`Compiling group "${name}"`, def);
    }

    const selector = this.compile(def);
    this.compiled.set(name, selector);
    return selector;
  }

  private compile(def: GroupDef): GroupSelector {
    switch (def.kind) {
      case 'query': {
        const q = def.query;
        return state => runQuery(state, q);
      }

      case 'not': {
        return state => {
          const base = this.eval(def.ref, state);
          const all = allItemIds(state);
          // complement in universe
          const out = new Set<string>();
          for (const id of all) if (!base.has(id)) out.add(id);
          return out;
        };
      }

      case 'op': {
        return state => {
          const sets = def.refs.map(r => this.eval(r, state));
          if (def.op === 'or') {
            const out = new Set<string>();
            sets.forEach(s => s.forEach(id => out.add(id)));
            return out;
          }
          // and
          if (sets.length === 0) return new Set();
          const out = new Set<string>(sets[0]);
          for (let i = 1; i < sets.length; i++) {
            const current = sets[i]!;
            for (const id of Array.from(out)) if (!current.has(id)) out.delete(id);
          }
          return out;
        };
      }
    }
  }
}

function runQuery(state: AppState, q: ItemQuery): Set<string> {
  switch (q.kind) {
    case 'lastOfAllSections': {
      // “last item in all of appState.sections”
      const items = allItems(state);
      if (items.length === 0) return new Set();
      // define “last” as max index
      const last = items.reduce<AudioFileItem>((a, b) => (b.index > a.index ? b : a), items[0]!);
      return new Set([last.id]);
    }

    case 'lastOfEachSection': {
      const out = new Set<string>();
      for (const sec of state.sections) {
        if (sec.files.length === 0) continue;
        const last = sec.files.reduce<AudioFileItem>(
          (a, b) => (b.index > a.index ? b : a),
          sec.files[0]!
        );
        out.add(last.id);
      }
      return out;
    }

    case 'sectionPercent': {
      const sec = state.sections[q.sectionIndex];
      if (!sec) return new Set();

      const files = [...sec.files];
      const orderBy = q.orderBy ?? 'index';
      files.sort((a, b) => (a[orderBy] ?? 0) - (b[orderBy] ?? 0));

      const count = Math.max(0, Math.floor(files.length * clamp01(q.percent)));
      const take = q.take ?? 'first';
      const picked = take === 'first' ? files.slice(0, count) : files.slice(-count);

      return new Set(picked.map(f => f.id));
    }

    case 'where': {
      const out = new Set<string>();
      for (const f of allItems(state)) {
        if (matchesWhere(f, q.clause)) out.add(f.id);
      }
      return out;
    }
  }
}

function clamp01(x: number) {
  return Math.min(1, Math.max(0, x));
}

function matchesWhere(f: AudioFileItem, c: WhereClause): boolean {
  switch (c.field) {
    case 'active':
      return f.active === c.eq;
    case 'color':
      return f.color === c.eq;
    case 'duration': {
      const d = f.duration ?? 0;
      if (c.gt != null && !(d > c.gt)) return false;
      if (c.lt != null && !(d < c.lt)) return false;
      return true;
    }
    case 'path':
      return f.path.includes(c.includes);
  }
}

function allItems(state: AppState): AudioFileItem[] {
  return state.sections.flatMap(s => s.files);
}

function allItemIds(state: AppState): Set<string> {
  return new Set(allItems(state).map(f => f.id));
}

type NamedGroupDef = {
  name: string;
  def: GroupDef;
};

export const testGroups: NamedGroupDef[] = [
  {
    name: 'sec0_half',
    def: {
      kind: 'query',
      query: {
        kind: 'sectionPercent',
        sectionIndex: 0,
        percent: 0.5,
        take: 'first',
        orderBy: 'index',
      },
    },
  },

  {
    name: 'global_last',
    def: {
      kind: 'query',
      query: {
        kind: 'lastOfAllSections',
      },
    },
  },

  {
    name: 'active_only',
    def: {
      kind: 'query',
      query: {
        kind: 'where',
        clause: {
          field: 'active',
          eq: true,
        },
      },
    },
  },

  {
    name: 'half_or_last',
    def: {
      kind: 'op',
      op: 'or',
      refs: ['sec0_half', 'global_last'],
    },
  },

  {
    name: 'combo',
    def: {
      kind: 'op',
      op: 'and',
      refs: ['active_only', 'half_or_last'],
    },
  },
];

export const groupRegistry = new GroupRegistry(() => {
  return get(appState).groups?.defs;
});

let lastRev = get(appState)._rev ?? 0;

appState.subscribe(state => {
  const currentRev = state._rev ?? 0;
  const isLogging = get(loggingState).groupsLog;

  if (isLogging) {
    console.log(`%cHERE LINE :264 %c`, 'color: yellow; font-weight: bold', '');
    logger.groups.info(`AppState revision check: current=${currentRev}, last=${lastRev}`);
  }

  // If content revision changed, cached group results are invalid
  if (currentRev !== lastRev) {
    if (isLogging) {
      logger.groups.warning(`Content revision changed from ${lastRev} to ${currentRev} - invalidating cache`);
    }
    
    groupRegistry.invalidateAll();
    lastRev = currentRev;
  }
});
