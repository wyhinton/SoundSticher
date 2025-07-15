import type { Section } from "$lib/state/state.svelte";

export type AbletonColor = {
  name: string;
  rgb: [number, number, number];
};

export const ABLETON_COLORS: AbletonColor[] = [
  { name: "Red", rgb: [255, 64, 64] },
  { name: "Orange", rgb: [255, 128, 0] },
  { name: "Yellow", rgb: [255, 221, 0] },
  { name: "Light Green", rgb: [128, 255, 0] },
  { name: "Green", rgb: [0, 191, 0] },
  { name: "Cyan", rgb: [0, 255, 221] },
  { name: "Light Blue", rgb: [0, 128, 255] },
  { name: "Blue", rgb: [0, 0, 255] },
  { name: "Purple", rgb: [170, 0, 255] },
  { name: "Magenta", rgb: [255, 0, 221] },
  { name: "Pink", rgb: [255, 128, 170] },
  { name: "Brown", rgb: [153, 102, 51] },
  { name: "Gray", rgb: [153, 153, 153] },
  { name: "Light Gray", rgb: [204, 204, 204] },
  { name: "Dark Gray", rgb: [85, 85, 85] },
  { name: "White", rgb: [255, 255, 255] },
];


export function getNextAvailableColor(sections: Section[]): AbletonColor {
  const usedColors = new Set(sections.map(section => section.color.name));
  
  for (const color of ABLETON_COLORS) {
    if (!usedColors.has(color.name)) {
      return color;
    }
  }

  // Fallback: if all colors are used, just reuse the first one (or random)
  return ABLETON_COLORS[0];
}



export function toCssRgb(rgb: [number, number, number], alpha: number = 1): string {
  return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${alpha})`;
}

  
