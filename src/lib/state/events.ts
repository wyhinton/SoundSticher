import type { Channel } from "@tauri-apps/api/core";

export type BufferAudioEvent =
  | {
      event: 'started';
      data: {
        contentLength: number;
      };
    }
  | {
      event: 'progress';
      data: {
        chunkLength: number;
      };
    }
  | {
      event: 'finished';
      data: {
        downloadId: number;
      };
    };
  
export type CombineAudioEvent =
  | {
      event: 'started';
      data: {
        contentLength: number;
        duration: number;
      };
    }
  | {
      event: 'progress';
      data: {
        id: string,
        svgPath: string;
        startOffset: number;
        fileName: string;
        size: number;
      };
    }
  | {
      event: 'finished';
      data: {
        svgPath: string;
      };
    };


export type ExportAudioEvent =
  | {
      event: 'started';
      data: {
        outputPath: string;
      };
    }
  | {
      event: 'progress';
      data: {
        progress: number;
        message: string;
      };
    }
  | {
      event: 'finished';
      data: {
          outputPath: string;
      };
    };


export type SortAudioEvent =
  | {
      event: 'started';
      data: {
        outputPath: string;
      };
    }
  | {
      event: 'progress';
      data: {
        id: string;
        progress: number;
        startOffset: number;
      };
    }
  | {
      event: 'finished';
      data: {
          outputPath: string;
      };
    };



    type ChannelEventType = "started" | "progress" | "finished";

interface BaseEvent<T = any> {
  event: ChannelEventType;
  data: T;
}


export function generateProgressChannel<E extends { event: string; data: any }>(
  ChannelCtor: new () => Channel<E>,
  handlers: {
    [K in E as K["event"]]?: (data: Extract<E, { event: K["event"] }>["data"]) => void;
  }
): Channel<E> {
  const channel = new ChannelCtor();

  channel.onmessage = (message: E) => {
    const handler = handlers[message.event];
    if (handler) {
      // assert correct function type based on message.event
      (handler as (data: typeof message.data) => void)(message.data);
    }
  };

  return channel;
}