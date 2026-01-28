/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * Generated from JSON Schemas in /schemas/operations/
 * Generated at: 2026-01-26T21:48:31.202Z
 */


/** Parameters for ExportOp operation */
export interface ExportOpParams {
  format: "wav" | "mp3" | "flac" | "ogg" | "m4a" | "aac";
  quality: "low" | "medium" | "high" | "lossless";
  sample_rate?: 8000 | 11025 | 16000 | 22050 | 44100 | 48000 | 88200 | 96000;
  bit_depth?: 8 | 16 | 24 | 32;
  bit_rate?: 64 | 96 | 128 | 160 | 192 | 224 | 256 | 320;
  normalize_before_export?: boolean;
  metadata?: {
  title?: string;
  artist?: string;
  album?: string;
  year?: number;
  genre?: string;
  comment?: string;
};
}

/** Parameters for MergeOp operation */
export interface MergeOpParams {
  sample_rate: 8000 | 11025 | 16000 | 22050 | 44100 | 48000 | 88200 | 96000;
  bit_depth: 8 | 16 | 24 | 32;
  format: "wav" | "mp3" | "flac" | "ogg" | "m4a";
}

/** Parameters for PipelineOp operation */
export interface PipelineOpParams {
  parallel?: boolean;
  continue_on_error?: boolean;
  cache_intermediates?: boolean;
}

/** Parameters for SampleOp operation */
export interface SampleOpParams {
  trim_start_ms?: number;
  trim_end_ms?: number;
  fade_in_ms?: number;
  fade_out_ms?: number;
  gain_db?: number;
  reverse?: boolean;
}

/** Union of all operation parameter types */
export type OperationParams = ExportOpParams | MergeOpParams | PipelineOpParams | SampleOpParams;

/** All supported operation kinds */
export type OperationKind = "export" | "merge" | "pipeline" | "sample";

/** Default parameter values for each operation type */
export const operationDefaults: Record<OperationKind, Record<string, unknown>> = {
  export: {"format":"wav","quality":"high","sample_rate":44100,"bit_depth":16,"bit_rate":320,"normalize_before_export":false},
  merge: {"sample_rate":44100,"bit_depth":16,"format":"wav"},
  pipeline: {"parallel":false,"continue_on_error":false,"cache_intermediates":false},
  sample: {"trim_start_ms":0,"trim_end_ms":0,"fade_in_ms":0,"fade_out_ms":0,"gain_db":0,"reverse":false},
};

/** UI control configuration for each operation type */
export interface UIControl {
  key: string;
  type: string;
  label: string;
  description?: string;
  default?: unknown;
  group?: string;
  options?: (string | number)[];
  min?: number;
  max?: number;
  step?: number;
  placeholder?: string;
  showIf?: Record<string, unknown>;
}

export const operationUIControls: Record<OperationKind, UIControl[]> = {
  export: [
    {
      "key": "format",
      "type": "select",
      "label": "Format",
      "description": "Output audio format",
      "default": "wav",
      "group": "format",
      "options": [
        "wav",
        "mp3",
        "flac",
        "ogg",
        "m4a",
        "aac"
      ]
    },
    {
      "key": "quality",
      "type": "select",
      "label": "Quality",
      "description": "Encoding quality preset",
      "default": "high",
      "group": "format",
      "options": [
        "low",
        "medium",
        "high",
        "lossless"
      ]
    },
    {
      "key": "sample_rate",
      "type": "select",
      "label": "Sample Rate (Hz)",
      "description": "Output sample rate in Hz",
      "default": 44100,
      "group": "format",
      "options": [
        8000,
        11025,
        16000,
        22050,
        44100,
        48000,
        88200,
        96000
      ]
    },
    {
      "key": "bit_depth",
      "type": "select",
      "label": "Bit Depth",
      "description": "Output bit depth (for lossless formats)",
      "default": 16,
      "group": "format",
      "showIf": {
        "format": [
          "wav",
          "flac"
        ]
      },
      "options": [
        8,
        16,
        24,
        32
      ]
    },
    {
      "key": "bit_rate",
      "type": "select",
      "label": "Bit Rate (kbps)",
      "description": "Bit rate in kbps (for lossy formats)",
      "default": 320,
      "group": "format",
      "showIf": {
        "format": [
          "mp3",
          "ogg",
          "m4a",
          "aac"
        ]
      },
      "options": [
        64,
        96,
        128,
        160,
        192,
        224,
        256,
        320
      ]
    },
    {
      "key": "normalize_before_export",
      "type": "checkbox",
      "label": "Normalize Before Export",
      "description": "Normalize audio before exporting",
      "default": false,
      "group": "processing"
    },
    {
      "key": "metadata",
      "type": "metadata-editor",
      "label": "Metadata",
      "description": "Metadata to embed in the output file",
      "group": "metadata"
    }
  ],
  merge: [
    {
      "key": "sample_rate",
      "type": "select",
      "label": "Sample Rate (Hz)",
      "description": "Output sample rate in Hz",
      "default": 44100,
      "group": "audio",
      "options": [
        8000,
        11025,
        16000,
        22050,
        44100,
        48000,
        88200,
        96000
      ]
    },
    {
      "key": "bit_depth",
      "type": "select",
      "label": "Bit Depth",
      "description": "Output bit depth",
      "default": 16,
      "group": "audio",
      "options": [
        8,
        16,
        24,
        32
      ]
    },
    {
      "key": "format",
      "type": "select",
      "label": "Output Format",
      "description": "Output audio format",
      "default": "wav",
      "group": "audio",
      "options": [
        "wav",
        "mp3",
        "flac",
        "ogg",
        "m4a"
      ]
    }
  ],
  pipeline: [
    {
      "key": "parallel",
      "type": "checkbox",
      "label": "Parallel Execution",
      "description": "Execute independent operations in parallel when possible",
      "default": false,
      "group": "execution"
    },
    {
      "key": "continue_on_error",
      "type": "checkbox",
      "label": "Continue on Error",
      "description": "Continue pipeline even if an operation fails",
      "default": false,
      "group": "execution"
    },
    {
      "key": "cache_intermediates",
      "type": "checkbox",
      "label": "Cache Intermediates",
      "description": "Cache intermediate results for debugging",
      "default": false,
      "group": "execution"
    }
  ],
  sample: [
    {
      "key": "trim_start_ms",
      "type": "number",
      "label": "Trim Start (ms)",
      "description": "Trim from start in milliseconds",
      "default": 0,
      "group": "trim",
      "min": 0
    },
    {
      "key": "trim_end_ms",
      "type": "number",
      "label": "Trim End (ms)",
      "description": "Trim from end in milliseconds",
      "default": 0,
      "group": "trim",
      "min": 0
    },
    {
      "key": "fade_in_ms",
      "type": "slider",
      "label": "Fade In (ms)",
      "description": "Fade in duration in milliseconds",
      "default": 0,
      "group": "fades",
      "min": 0,
      "max": 10000,
      "step": 10
    },
    {
      "key": "fade_out_ms",
      "type": "slider",
      "label": "Fade Out (ms)",
      "description": "Fade out duration in milliseconds",
      "default": 0,
      "group": "fades",
      "min": 0,
      "max": 10000,
      "step": 10
    },
    {
      "key": "gain_db",
      "type": "slider",
      "label": "Gain (dB)",
      "description": "Gain adjustment in dB",
      "default": 0,
      "group": "levels",
      "min": -60,
      "max": 24,
      "step": 0.5
    },
    {
      "key": "reverse",
      "type": "checkbox",
      "label": "Reverse",
      "description": "Reverse the audio",
      "default": false,
      "group": "effects"
    }
  ],
};