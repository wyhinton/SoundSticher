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