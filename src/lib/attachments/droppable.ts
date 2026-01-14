import { DragPayload } from './draggable';

const MIME = 'application/x-svelte-dnd';

export interface DropzoneOptions<T = unknown> {
  accepts?: string[];
  dropEffect?: DataTransfer['dropEffect'];
  dragover_class?: string;
  on_drop: (payload: DragPayload<T>, event: DragEvent) => void;
}

export function dropzone<T>(node: HTMLElement, options: DropzoneOptions<T>) {
  node.addEventListener('dragover', e => {
    console.log('dragover firing');
    e.preventDefault();
  });
  node.draggable = true;
  let state = {
    dropEffect: options.dropEffect ?? 'move',
    dragover_class: options.dragover_class ?? 'droppable',
    on_drop: options.on_drop,
    accepts: options.accepts,
  };

  function handle_dragenter() {
    node.classList.add(state.dragover_class);
  }

  function handle_dragleave() {
    node.classList.remove(state.dragover_class);
  }

  function handle_dragover(e: DragEvent) {
    // 🚨 REQUIRED or drop will never fire
    e.preventDefault();

    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = state.dropEffect;
    }
  }

  function handle_drop(e: DragEvent) {
    e.preventDefault();
    node.classList.remove(state.dragover_class);

    if (!e.dataTransfer) return;

    const raw = e.dataTransfer.getData(MIME);
    if (!raw) return;

    let payload: DragPayload<T>;
    try {
      payload = JSON.parse(raw);
    } catch {
      return;
    }

    if (state.accepts && !state.accepts.includes(payload.type)) return;

    state.on_drop(payload, e);
  }

  node.addEventListener('dragenter', handle_dragenter);
  node.addEventListener('dragleave', handle_dragleave);
  node.addEventListener('dragover', handle_dragover);
  node.addEventListener('drop', handle_drop);

  return {
    update(options: DropzoneOptions<T>) {
      state = {
        dropEffect: options.dropEffect ?? 'move',
        dragover_class: options.dragover_class ?? 'droppable',
        on_drop: options.on_drop,
        accepts: options.accepts,
      };
    },

    destroy() {
      node.removeEventListener('dragenter', handle_dragenter);
      node.removeEventListener('dragleave', handle_dragleave);
      node.removeEventListener('dragover', handle_dragover);
      node.removeEventListener('drop', handle_drop);
    },
  };
}
