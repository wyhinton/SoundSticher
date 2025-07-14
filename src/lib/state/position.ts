import type { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import { Webview } from "@tauri-apps/api/webview";
import { persisted } from "svelte-persisted-store";
import { performanceStore } from "./performance";

interface PositionState {
  viewPosition?: {x: number, y: number};
  viewSize?: {width: number, height: number};
  innerPosition?: {x: number, y: number};
  innerSize?: {width: number, height: number};
}

export const positionStore = persisted<PositionState>(
  "positionState",
  {
    innerPosition: undefined,
    viewSize: undefined,
    innerSize: undefined,
    viewPosition: undefined,
  }
);

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
  // positionStore.update((s)=>{
  //   view.position().
  // })
  //  Promise.all([
  //   view.position(),
  //   view.size(),
  //   view.window.innerPosition(),
  //   view.window.innerSize(),
  // ]).then(([position, size, innerPosition, innerSize]) => {
  //   positionStore.update((store)=>({
  //       ...store,
  //       viewPosition: position.toJSON(), 
  //       viewSize: size.toJSON(),
  //       innerPosition: innerPosition.toJSON(),
  //       innerSize: innerSize.toJSON()        
  //   }))
  // });

  // view.position().then((p) => {
  //   console.log(`POSITION: ${p}`);
  // });
  // view.size().then((p) => {
  //   console.log(`SIZE: ${p}`);
  // });
  // view.window.innerPosition().then((p) => {
  //   console.log(`INNER POSITION: ${p}`);
  // });
  // view.window.innerSize().then((p) => {
  //   console.log(`INNER SIZE: ${p}`);
  // });
};

positionStore.subscribe((s)=>{
  console.log(s)
})
