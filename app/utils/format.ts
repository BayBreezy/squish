export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes)) return "–";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

export function formatPercent(value: number): string {
  const rounded = Math.round(value);
  return rounded > 0 ? `-${rounded}%` : rounded < 0 ? `+${Math.abs(rounded)}%` : "0%";
}
