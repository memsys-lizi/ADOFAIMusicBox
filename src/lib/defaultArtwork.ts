import defaultCover from "../assets/header.jpg";
import defaultIcon from "../assets/icon.jpg";
import defaultRdCover from "../assets/header_rd.jpg";
import defaultRdIcon from "../assets/icon_rd.jpg";
import type { GameMode } from "../types/domain";

export type ArtworkFallback = "icon" | "cover";

export function defaultArtworkSource(game: GameMode | null | undefined, fallback: ArtworkFallback) {
  if (game === "rhythmDoctor") {
    return fallback === "icon" ? defaultRdIcon : defaultRdCover;
  }
  return fallback === "icon" ? defaultIcon : defaultCover;
}
