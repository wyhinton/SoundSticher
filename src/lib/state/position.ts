import type { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import { Webview } from "@tauri-apps/api/webview";
import { persisted } from "svelte-persisted-store";
import { performanceStore } from "./performance";
import { derived } from "svelte/store";

interface PositionState {
  inputsUnderMouse: number[],
  viewPosition?: {x: number, y: number};
  viewSize?: {width: number, height: number};
  innerPosition?: {x: number, y: number};
  innerSize?: {width: number, height: number};
  isOverTableContainer: boolean;
}

export const positionStore = persisted<PositionState>(
  "positionState",
  {
    innerPosition: undefined,
    viewSize: undefined,
    innerSize: undefined,
    viewPosition: undefined,
    inputsUnderMouse: [],
    isOverTableContainer: false,
  },
);

export const setIsOverTableContainer = (isOver: boolean) =>{
  positionStore.update(s=>{
    s.isOverTableContainer = isOver;
    return s;
  })
}

export const setInputsUnderMouse = (inputs: number[]) =>{
  positionStore.update(s=>{
    s.inputsUnderMouse = inputs;
    return s;
  })
}

export const clearUnderMouse = () =>{
  positionStore.update((s)=>{
    s.inputsUnderMouse = [];
    s.isOverTableContainer = false;
    return s;
  })
}

export const logPositionInfo = (view: Webview) => {
  console.log();

    view.position().then((p)=>{
      positionStore.update((s)=>{
        s.viewPosition = p.toJSON()
        return s;
      })
    })
    view.window.innerPosition().then((p)=>{
      positionStore.update((s)=>{
        s.innerPosition = p.toJSON()
        return s;
      })
    })
};

export const addNewFolderOnDrop = derived(positionStore, (s)=>{
  return s.inputsUnderMouse.length === 0 && s.isOverTableContainer;
})

