import { Disc3 } from "lucide-react";
import type { CSSProperties } from "react";
import { fallbackCoverSeed, toAssetUrl } from "../lib/assets";

interface EmptyArtworkProps {
  title: string;
  imagePath?: string | null;
  size?: "sm" | "md" | "lg";
}

export function EmptyArtwork({ title, imagePath, size = "md" }: EmptyArtworkProps) {
  const image = toAssetUrl(imagePath);
  if (image) {
    return <img className={`artwork artwork-${size}`} src={image} alt={`${title} 封面`} />;
  }

  return (
    <div
      className={`artwork artwork-${size} generated-artwork`}
      style={{ "--cover-angle": fallbackCoverSeed(title) } as CSSProperties}
      aria-label={`${title} 默认封面`}
    >
      <Disc3 aria-hidden="true" />
    </div>
  );
}
