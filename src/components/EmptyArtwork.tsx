import defaultCover from "../assets/header.jpg";
import defaultIcon from "../assets/icon.jpg";
import { toAssetUrl } from "../lib/assets";

interface EmptyArtworkProps {
  title: string;
  imagePath?: string | null;
  size?: "sm" | "md" | "lg";
  fallback?: "icon" | "cover";
}

export function EmptyArtwork({ title, imagePath, size = "md", fallback }: EmptyArtworkProps) {
  const fallbackKind = fallback ?? (size === "sm" ? "icon" : "cover");
  const image = toAssetUrl(imagePath) ?? (fallbackKind === "icon" ? defaultIcon : defaultCover);
  return <img className={`artwork artwork-${size}`} src={image} alt={`${title} 封面`} />;
}
