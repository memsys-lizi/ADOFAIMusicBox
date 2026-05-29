import { useEffect, useState } from "react";
import { toAssetUrl } from "../lib/assets";

export interface CoverPalette {
  accent: string;
  accentText: string;
  backgroundA: string;
  backgroundB: string;
  soft: string;
}

const DEFAULT_PALETTE: CoverPalette = {
  accent: "#d1cc2e",
  accentText: "#11120d",
  backgroundA: "#fffbd5",
  backgroundB: "#ecffd7",
  soft: "#dfe8a8",
};

export function useCoverPalette(imagePath?: string | null): CoverPalette {
  const [palette, setPalette] = useState<CoverPalette>(DEFAULT_PALETTE);

  useEffect(() => {
    const source = toAssetUrl(imagePath);
    if (!source) {
      setPalette(DEFAULT_PALETTE);
      return;
    }

    let cancelled = false;
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        const size = 32;
        canvas.width = size;
        canvas.height = size;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        if (!context) {
          throw new Error("canvas unavailable");
        }
        context.drawImage(image, 0, 0, size, size);
        const data = context.getImageData(0, 0, size, size).data;
        let red = 0;
        let green = 0;
        let blue = 0;
        let count = 0;

        for (let index = 0; index < data.length; index += 4) {
          const alpha = data[index + 3];
          if (alpha < 64) {
            continue;
          }
          red += data[index];
          green += data[index + 1];
          blue += data[index + 2];
          count += 1;
        }

        if (count === 0) {
          throw new Error("empty image");
        }

        const hsl = rgbToHsl(red / count, green / count, blue / count);
        const next: CoverPalette = {
          accent: hslToCss(hsl.h, Math.max(46, hsl.s), clamp(hsl.l, 42, 58)),
          accentText: hsl.l > 55 ? "#161713" : "#ffffff",
          backgroundA: hslToCss(hsl.h, Math.max(50, hsl.s), 93),
          backgroundB: hslToCss((hsl.h + 36) % 360, Math.max(42, hsl.s - 4), 89),
          soft: hslToCss(hsl.h, Math.max(38, hsl.s), 78),
        };
        if (!cancelled) {
          setPalette(next);
        }
      } catch {
        if (!cancelled) {
          setPalette(DEFAULT_PALETTE);
        }
      }
    };
    image.onerror = () => {
      if (!cancelled) {
        setPalette(DEFAULT_PALETTE);
      }
    };
    image.src = source;

    return () => {
      cancelled = true;
    };
  }, [imagePath]);

  return palette;
}

function rgbToHsl(red: number, green: number, blue: number) {
  const r = red / 255;
  const g = green / 255;
  const b = blue / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const lightness = (max + min) / 2;
  const delta = max - min;

  if (delta === 0) {
    return { h: 72, s: 28, l: lightness * 100 };
  }

  const saturation = delta / (1 - Math.abs(2 * lightness - 1));
  let hue = 0;
  if (max === r) {
    hue = ((g - b) / delta) % 6;
  } else if (max === g) {
    hue = (b - r) / delta + 2;
  } else {
    hue = (r - g) / delta + 4;
  }
  hue *= 60;
  if (hue < 0) {
    hue += 360;
  }

  return { h: hue, s: saturation * 100, l: lightness * 100 };
}

function hslToCss(hue: number, saturation: number, lightness: number) {
  return `hsl(${Math.round(hue)} ${Math.round(saturation)}% ${Math.round(lightness)}%)`;
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}
