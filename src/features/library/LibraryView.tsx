import {
  ChevronDown,
  CheckCircle2,
  CircleAlert,
  FileMusic,
  FolderPlus,
  Grid2X2,
  Heart,
  List,
  LocateFixed,
  MoreHorizontal,
  Play,
  Plus,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { EmptyArtwork } from "../../components/EmptyArtwork";
import { formatDuration, formatFileSize } from "../../lib/format";
import type { AppView, TrackSummary } from "../../types/domain";

interface LibraryViewProps {
  activeView: AppView;
  tracks: TrackSummary[];
  allTrackCount: number;
  favoriteCount: number;
  recentCount: number;
  selectedTrack: TrackSummary | null;
  favoriteIds: Set<string>;
  query: string;
  isScanning: boolean;
  locateRequest: number;
  canLocatePlaying: boolean;
  onViewChange: (view: AppView) => void;
  onPlayTrack: (track: TrackSummary) => void;
  onLocatePlaying: () => void;
  onPlayAll: () => void;
  onToggleFavorite: (track: TrackSummary) => void;
  onAddFolder: () => void;
  onAddFile: () => void;
  onOpenFolderManager: () => void;
  onScan: () => void;
}

type LayoutMode = "list" | "grid";
type SortMode = "title" | "artist" | "duration" | "bpm";

const sortOptions: Array<{ value: SortMode; label: string }> = [
  { value: "title", label: "按歌名" },
  { value: "artist", label: "按作曲" },
  { value: "duration", label: "按时长" },
  { value: "bpm", label: "按 BPM" },
];

export function LibraryView({
  activeView,
  tracks,
  allTrackCount,
  favoriteCount,
  recentCount,
  selectedTrack,
  favoriteIds,
  query,
  isScanning,
  locateRequest,
  canLocatePlaying,
  onViewChange,
  onPlayTrack,
  onLocatePlaying,
  onPlayAll,
  onToggleFavorite,
  onAddFolder,
  onAddFile,
  onOpenFolderManager,
  onScan,
}: LibraryViewProps) {
  const [layout, setLayout] = useState<LayoutMode>("list");
  const [sortMode, setSortMode] = useState<SortMode>("title");
  const [menuOpen, setMenuOpen] = useState(false);
  const [addMenuOpen, setAddMenuOpen] = useState(false);
  const [sortOpen, setSortOpen] = useState(false);
  const itemRefs = useRef(new Map<string, HTMLDivElement>());

  const shownTracks = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    const filtered = normalized
      ? tracks.filter((track) =>
          [track.title, track.artist, track.author, track.folderPath]
            .join(" ")
            .toLowerCase()
            .includes(normalized),
        )
      : tracks;

    return [...filtered].sort((a, b) => compareTracks(a, b, sortMode));
  }, [query, sortMode, tracks]);

  useEffect(() => {
    if (!locateRequest || !selectedTrack) {
      return;
    }

    const node = itemRefs.current.get(selectedTrack.id);
    if (!node) {
      return;
    }

    node.scrollIntoView({ block: "center", behavior: "smooth" });
    node.classList.remove("locate-pulse");
    window.requestAnimationFrame(() => node.classList.add("locate-pulse"));
    const timer = window.setTimeout(() => node.classList.remove("locate-pulse"), 1100);
    return () => window.clearTimeout(timer);
  }, [locateRequest]);

  function setTrackRef(trackId: string, node: HTMLDivElement | null) {
    if (node) {
      itemRefs.current.set(trackId, node);
    } else {
      itemRefs.current.delete(trackId);
    }
  }

  return (
    <section className="library-page">
      <header className="library-header">
        <div>
          <h1>{viewTitle(activeView)}</h1>
          <div className="library-tabs">
            <button
              className={activeView === "local" ? "active" : ""}
              type="button"
              onClick={() => onViewChange("local")}
            >
              本地曲目{allTrackCount}
            </button>
            <button
              className={activeView === "favorites" ? "active" : ""}
              type="button"
              onClick={() => onViewChange("favorites")}
            >
              喜欢{favoriteCount}
            </button>
            <button
              className={activeView === "recent" ? "active" : ""}
              type="button"
              onClick={() => onViewChange("recent")}
            >
              最近播放{recentCount}
            </button>
          </div>
        </div>
        <div className="library-summary">
          <span>{shownTracks.length} 首</span>
          <span>{isScanning ? "扫描中" : "已就绪"}</span>
        </div>
      </header>

      <div className="library-actions">
        <button className="pill-button main" type="button" onClick={onPlayAll} disabled={shownTracks.length === 0}>
          <Play aria-hidden="true" />
          <span>播放</span>
        </button>
        <div className="more-wrap">
          <button
            className="pill-button"
            type="button"
            onClick={() => setAddMenuOpen((open) => !open)}
          >
            <Plus aria-hidden="true" />
            <span>添加</span>
          </button>
          {addMenuOpen && (
            <div className="more-menu">
              <button
                type="button"
                onClick={() => {
                  setAddMenuOpen(false);
                  onAddFolder();
                }}
              >
                <FolderPlus aria-hidden="true" />
                添加文件夹
              </button>
              <button
                type="button"
                onClick={() => {
                  setAddMenuOpen(false);
                  onAddFile();
                }}
              >
                <FileMusic aria-hidden="true" />
                添加单曲
              </button>
            </div>
          )}
        </div>
        <div className="more-wrap">
          <button
            className="round-button"
            type="button"
            onClick={() => setMenuOpen((open) => !open)}
            title="更多"
          >
            <MoreHorizontal aria-hidden="true" />
          </button>
          {menuOpen && (
            <div className="more-menu">
              <button
                type="button"
                onClick={() => {
                  setMenuOpen(false);
                  onOpenFolderManager();
                }}
              >
                管理来源
              </button>
              <button
                type="button"
                onClick={() => {
                  setMenuOpen(false);
                  onScan();
                }}
              >
                重新扫描
              </button>
            </div>
          )}
        </div>

        <div className="library-tools">
          <div className="sort-wrap">
            <button className="sort-button" type="button" onClick={() => setSortOpen((open) => !open)}>
              <span>{sortLabel(sortMode)}</span>
              <ChevronDown aria-hidden="true" />
            </button>
            {sortOpen && (
              <div className="sort-menu">
                {sortOptions.map((option) => (
                  <button
                    className={option.value === sortMode ? "active" : ""}
                    key={option.value}
                    type="button"
                    onClick={() => {
                      setSortMode(option.value);
                      setSortOpen(false);
                    }}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            )}
          </div>
          <button
            className={layout === "list" ? "tool-button active" : "tool-button"}
            type="button"
            onClick={() => setLayout("list")}
            title="列表"
          >
            <List aria-hidden="true" />
          </button>
          <button
            className={layout === "grid" ? "tool-button active" : "tool-button"}
            type="button"
            onClick={() => setLayout("grid")}
            title="封面"
          >
            <Grid2X2 aria-hidden="true" />
          </button>
        </div>
      </div>

      {shownTracks.length === 0 ? (
        <div className="empty-library">
          <div className="empty-icon">
            {activeView === "local" ? <Plus aria-hidden="true" /> : <Heart aria-hidden="true" />}
          </div>
          <h2>{emptyTitle(activeView, query)}</h2>
          <p>{emptyDescription(activeView, query)}</p>
          {activeView === "local" && (
            <button className="pill-button main" type="button" onClick={onAddFolder}>
              <Plus aria-hidden="true" />
              <span>添加文件夹</span>
            </button>
          )}
        </div>
      ) : layout === "list" ? (
        <div className="track-table">
          <div className="track-head">
            <span>歌名/作曲</span>
            <span>谱面</span>
            <span>时长</span>
            <span>大小</span>
          </div>
          {shownTracks.map((track) => (
            <TrackRow
              key={track.id}
              track={track}
              selected={selectedTrack?.id === track.id}
              favorite={favoriteIds.has(track.id)}
              trackRef={(node) => setTrackRef(track.id, node)}
              onPlayTrack={onPlayTrack}
              onToggleFavorite={onToggleFavorite}
            />
          ))}
        </div>
      ) : (
        <div className="cover-grid">
          {shownTracks.map((track) => (
            <TrackCard
              key={track.id}
              track={track}
              selected={selectedTrack?.id === track.id}
              favorite={favoriteIds.has(track.id)}
              trackRef={(node) => setTrackRef(track.id, node)}
              onPlayTrack={onPlayTrack}
              onToggleFavorite={onToggleFavorite}
            />
          ))}
        </div>
      )}

      {canLocatePlaying && (
        <button
          className="locate-playing-button"
          type="button"
          onClick={onLocatePlaying}
          title="定位当前播放"
        >
          <LocateFixed aria-hidden="true" />
        </button>
      )}
    </section>
  );
}

interface TrackItemProps {
  track: TrackSummary;
  selected: boolean;
  favorite: boolean;
  trackRef: (node: HTMLDivElement | null) => void;
  onPlayTrack: (track: TrackSummary) => void;
  onToggleFavorite: (track: TrackSummary) => void;
}

function TrackRow({ track, selected, favorite, trackRef, onPlayTrack, onToggleFavorite }: TrackItemProps) {
  return (
    <div
      ref={trackRef}
      className={selected ? "track-row selected" : "track-row"}
      role="button"
      tabIndex={0}
      onClick={() => onPlayTrack(track)}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          onPlayTrack(track);
        }
      }}
    >
      <div className="song-cell">
        <EmptyArtwork title={track.title} imagePath={track.iconPath ?? track.coverPath} size="sm" />
        <div>
          <strong>{track.title}</strong>
          <span>{track.artist}</span>
        </div>
      </div>
      <button
        className={favorite ? "heart-button active" : "heart-button"}
        type="button"
        title={favorite ? "取消喜欢" : "喜欢"}
        onClick={(event) => {
          event.stopPropagation();
          onToggleFavorite(track);
        }}
      >
        <Heart aria-hidden="true" />
      </button>
      <span className={track.audioPath ? "playable-state ready" : "playable-state missing"}>
        {track.audioPath ? <CheckCircle2 aria-hidden="true" /> : <CircleAlert aria-hidden="true" />}
      </span>
      <span className="mapper-cell">{track.author}</span>
      <span>{formatDuration(track.duration)}</span>
      <span>{formatFileSize(track.audioFileSize)}</span>
    </div>
  );
}

function TrackCard({ track, selected, favorite, trackRef, onPlayTrack, onToggleFavorite }: TrackItemProps) {
  return (
    <div ref={trackRef} className={selected ? "track-card selected" : "track-card"}>
      <button className="cover-play" type="button" onClick={() => onPlayTrack(track)}>
        <EmptyArtwork title={track.title} imagePath={track.coverPath} size="md" />
        <span>
          <Play aria-hidden="true" />
        </span>
      </button>
      <div className="card-meta">
        <strong>{track.title}</strong>
        <span>{track.artist}</span>
      </div>
      <button
        className={favorite ? "heart-button active" : "heart-button"}
        type="button"
        onClick={() => onToggleFavorite(track)}
        title={favorite ? "取消喜欢" : "喜欢"}
      >
        <Heart aria-hidden="true" />
      </button>
    </div>
  );
}

function compareTracks(a: TrackSummary, b: TrackSummary, mode: SortMode) {
  if (mode === "duration") {
    return b.duration - a.duration;
  }
  if (mode === "bpm") {
    return b.bpm - a.bpm;
  }
  const left = mode === "artist" ? a.artist : a.title;
  const right = mode === "artist" ? b.artist : b.title;
  return left.localeCompare(right, "zh-CN");
}

function sortLabel(mode: SortMode) {
  return sortOptions.find((option) => option.value === mode)?.label ?? "按歌名";
}

function viewTitle(view: AppView) {
  switch (view) {
    case "favorites":
      return "喜欢";
    case "recent":
      return "最近播放";
    case "local":
      return "本地";
  }
}

function emptyTitle(view: AppView, query: string) {
  if (query.trim()) {
    return "没有找到匹配的音乐";
  }
  if (view === "favorites") {
    return "还没有喜欢的音乐";
  }
  if (view === "recent") {
    return "还没有最近播放";
  }
  return "添加文件夹开始播放";
}

function emptyDescription(view: AppView, query: string) {
  if (query.trim()) {
    return "换一个关键词试试。";
  }
  if (view === "favorites") {
    return "点亮歌曲旁边的爱心后，会收在这里。";
  }
  if (view === "recent") {
    return "播放过的音乐会自动出现在这里。";
  }
  return "选择包含 .adofai 谱面的文件夹，软件会自动扫描音乐和封面。";
}
