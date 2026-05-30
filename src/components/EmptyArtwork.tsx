import { toAssetUrl } from "../lib/assets";
import { defaultArtworkSource, type ArtworkFallback } from "../lib/defaultArtwork";
import type { GameMode } from "../types/domain";

interface EmptyArtworkProps {
  title: string;
  imagePath?: string | null;
  game?: GameMode | null;
  size?: "sm" | "md" | "lg";
  fallback?: ArtworkFallback;
}

export function EmptyArtwork({ title, imagePath, game, size = "md", fallback }: EmptyArtworkProps) {
  const fallbackKind = fallback ?? (size === "sm" ? "icon" : "cover");
  const image = toAssetUrl(imagePath) ?? defaultArtworkSource(game, fallbackKind);
  return <img className={`artwork artwork-${size}`} src={image} alt={`${title} 封面`} />;
}
