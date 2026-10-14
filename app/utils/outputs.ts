import type { ConcreteFormat, OutputState, QueueItem } from "~/types/squish";

/** Display order: matches the order the backend encodes in (cheap → slow). */
export const FORMAT_ORDER: ConcreteFormat[] = ["png", "jpeg", "webp", "avif"];

export const FORMAT_LABEL: Record<ConcreteFormat, string> = {
  png: "PNG",
  jpeg: "JPEG",
  webp: "WebP",
  avif: "AVIF",
};

export type WrittenOutput = Extract<OutputState, { kind: "written" }> & { format: ConcreteFormat };

/** Outputs that were written to disk, in display order. */
export function writtenOutputs(item: QueueItem): WrittenOutput[] {
  return FORMAT_ORDER.flatMap((format) => {
    const o = item.outputs[format];
    return o?.kind === "written" ? [{ ...o, format }] : [];
  });
}

/** The smallest written output, or undefined when nothing was written. */
export function bestOutput(item: QueueItem): WrittenOutput | undefined {
  return writtenOutputs(item).reduce<WrittenOutput | undefined>(
    (best, o) => (!best || o.bytes < best.bytes ? o : best),
    undefined,
  );
}
