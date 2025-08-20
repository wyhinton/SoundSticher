export function formatBytes(bytes: number, decimals = 2): string {
  if (bytes === 0) return "0 Bytes";
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["Bytes", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i];
}

export function formatMilliseconds(ms: number): string {
  const seconds = ms / 1000;
  return `${seconds.toFixed(2)}s`;
}

export function formatFileName(filePath: string): string {
  return filePath.split(/[/\\]/).pop() || "";
}

export function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

export function toSource(obj) {
  return (
    JSON.stringify(
      obj,
      (key, value) => {
        if (typeof value === "string") {
          // Escape backslashes
          return value.replace(/\\/g, "\\\\");
        }
        return value;
      },
      2
    )
      // Replace double quotes with single quotes (optional)
      .replace(/"([^"]+)":/g, "'$1':")
      .replace(/"/g, "'")
  );
}
