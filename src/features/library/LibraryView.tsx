import { open } from "@tauri-apps/plugin-dialog";
import {
  Grid2X2,
  List,
  Plus,
  Search,
  SlidersHorizontal,
  Sparkles,
} from "lucide-react";
import { useMemo, useState } from "react";
import { EmptyArtwork } from "../../components/EmptyArtwork";
import { StatPill } from "../../components/StatPill";
import { formatCount, formatDuration } from "../../lib/format";
import type { LibrarySettings, TrackSummary } from "../../types/domain";

interface LibraryViewProps {
  tracks: TrackSummary[];
  settings: LibrarySettings | null;
  selectedTrack: TrackSummary | null;
  onAddFolder: (folder: string) => Promise<void>;
  onSelectTrack: (track: TrackSummary) => void;
  onPlayTrack: (track: TrackSummary) => void;
}

type LayoutMode = "list" | "grid";

export function LibraryView({
  tracks,
  settings,
  selectedTrack,
  onAddFolder,
  onSelectTrack,
  onPlayTrack,
}: LibraryViewProps) {
  const [query, setQuery] = useState("");
  const [layout, setLayout] = useState<LayoutMode>("list");
  const [statusFilter, setStatusFilter] = useState("all");

  const filteredTracks = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return tracks.filter((track) => {
      const matchQuery =
        !normalized ||
        [track.title, track.artist, track.author, track.folderPath]
          .join(" ")
          .toLowerCase()
          .includes(normalized);
      const matchStatus = statusFilter === "all" || track.parseStatus === statusFilter;
      return matchQuery && matchStatus;
    });
  }, [query, statusFilter, tracks]);

  async function chooseFolder() {
    const result = await open({ directory: true, multiple: false });
    if (typeof result === "string") {
      await onAddFolder(result);
    }
  }

  return (
    <section className="page library-page">
      <header className="page-header">
        <div>
          <p className="eyebrow">Library</p>
          <h2>谱面曲库</h2>
          <p className="page-description">收藏、搜索和播放带节拍音的 ADOFAI 音乐。</p>
        </div>
        <button className="primary-button" type="button" onClick={chooseFolder}>
          <Plus aria-hidden="true" />
          <span>添加文件夹</span>
        </button>
      </header>

      <div className="library-stats">
        <StatPill label="谱面" value={formatCount(tracks.length)} />
        <StatPill label="来源" value={formatCount(settings?.folders.length ?? 0)} />
        <StatPill
          label="可播放"
          value={formatCount(tracks.filter((track) => Boolean(track.audioPath)).length)}
        />
        <StatPill
          label="需留意"
          value={formatCount(tracks.filter((track) => track.parseStatus !== "ok").length)}
        />
      </div>

      <div className="toolbar">
        <label className="search-box">
          <Search aria-hidden="true" />
          <input
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder="搜索曲名、艺术家或谱师"
          />
        </label>
        <label className="select-box">
          <SlidersHorizontal aria-hidden="true" />
          <select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>
            <option value="all">全部曲目</option>
            <option value="ok">正常</option>
            <option value="lenient">兼容读取</option>
            <option value="warning">需留意</option>
            <option value="error">不可播放</option>
          </select>
        </label>
        <div className="segmented-control" aria-label="曲库布局">
          <button
            className={layout === "list" ? "active" : ""}
            type="button"
            onClick={() => setLayout("list")}
            title="列表"
          >
            <List aria-hidden="true" />
          </button>
          <button
            className={layout === "grid" ? "active" : ""}
            type="button"
            onClick={() => setLayout("grid")}
            title="网格"
          >
            <Grid2X2 aria-hidden="true" />
          </button>
        </div>
      </div>

      {tracks.length === 0 ? (
        <div className="empty-state">
          <Sparkles aria-hidden="true" />
          <h3>曲库是空的</h3>
          <p>添加你的 ADOFAI 音乐文件夹后，就可以在这里播放。</p>
          <button className="primary-button" type="button" onClick={chooseFolder}>
            <Plus aria-hidden="true" />
            <span>添加文件夹</span>
          </button>
        </div>
      ) : (
        <div className={layout === "grid" ? "track-grid" : "track-list"}>
          {filteredTracks.map((track) =>
            layout === "grid" ? (
              <TrackCard
                key={track.id}
                track={track}
                selected={selectedTrack?.id === track.id}
                onSelectTrack={onSelectTrack}
                onPlayTrack={onPlayTrack}
              />
            ) : (
              <TrackRow
                key={track.id}
                track={track}
                selected={selectedTrack?.id === track.id}
                onSelectTrack={onSelectTrack}
                onPlayTrack={onPlayTrack}
              />
            ),
          )}
        </div>
      )}
    </section>
  );
}

interface TrackItemProps {
  track: TrackSummary;
  selected: boolean;
  onSelectTrack: (track: TrackSummary) => void;
  onPlayTrack: (track: TrackSummary) => void;
}

function TrackRow({ track, selected, onSelectTrack, onPlayTrack }: TrackItemProps) {
  const playTrack = () => {
    onSelectTrack(track);
    onPlayTrack(track);
  };

  return (
    <button
      className={selected ? "track-row selected" : "track-row"}
      type="button"
      onClick={playTrack}
    >
      <EmptyArtwork title={track.title} imagePath={track.coverPath} size="sm" />
      <div className="track-main">
        <strong>{track.title}</strong>
        <span>{track.artist} · 谱师 {track.author}</span>
      </div>
      <span className="track-chip">{Math.round(track.bpm)} BPM</span>
      <span className="track-chip">{formatDuration(track.duration)}</span>
      <span className={`status-dot status-${track.parseStatus}`}>{statusLabel(track.parseStatus)}</span>
    </button>
  );
}

function TrackCard({ track, selected, onSelectTrack, onPlayTrack }: TrackItemProps) {
  const playTrack = () => {
    onSelectTrack(track);
    onPlayTrack(track);
  };

  return (
    <button
      className={selected ? "track-card selected" : "track-card"}
      type="button"
      onClick={playTrack}
    >
      <EmptyArtwork title={track.title} imagePath={track.coverPath} size="md" />
      <strong>{track.title}</strong>
      <span>{track.artist}</span>
      <div>
        <span className="track-chip">{Math.round(track.bpm)} BPM</span>
        <span className={`status-dot status-${track.parseStatus}`}>{statusLabel(track.parseStatus)}</span>
      </div>
    </button>
  );
}

function statusLabel(status: TrackSummary["parseStatus"]) {
  switch (status) {
    case "ok":
      return "标准";
    case "lenient":
      return "兼容";
    case "warning":
      return "留意";
    case "error":
      return "不可播放";
  }
}
