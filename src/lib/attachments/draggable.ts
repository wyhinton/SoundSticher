export interface DragPayload<T = unknown> {
  type: string;
  data: T;
  sourceId?: string;
}

const MIME = 'application/x-svelte-dnd';

export function draggable<T>(node: HTMLElement, payload: DragPayload<T>) {
  let state = payload;

  node.draggable = true;
  node.style.cursor = 'grab';

  function handle_dragstart(e: DragEvent) {
    console.log(`%cHERE LINE :16 %c`, 'color: yellow; font-weight: bold', '');
    console.log(e.dataTransfer);
    if (!e.dataTransfer) return;

    e.dataTransfer.setData(MIME, JSON.stringify(state));

    e.dataTransfer.effectAllowed = 'move';
  }

  node.addEventListener('dragstart', handle_dragstart);

  return {
    update(payload: DragPayload<T>) {
      state = payload;
    },

    destroy() {
      node.removeEventListener('dragstart', handle_dragstart);
    },
  };
}
