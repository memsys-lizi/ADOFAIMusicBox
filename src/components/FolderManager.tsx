import { FileMusic, FolderPlus, RefreshCw, RotateCcw, Search, Trash2, X } from "lucide-react";
import { useMemo, useState } from "react";
import type { GameMode, HiddenTrack } from "../types/domain";

interface FolderManagerProps {
  open: boolean;
  game: GameMode;
  folders: string[];
  files: string[];
  hiddenTracks: HiddenTrack[];
  isScanning: boolean;
  onClose: () => void;
  onAddFolder: () => void;
  onAddFile: () => void;
  onRemoveFolder: (folder: string) => void;
  onRemoveFile: (file: string) => void;
  onRestoreHiddenTrack: (track: HiddenTrack) => void;
  onScan: () => void;
}

type SourceTab = "folders" | "files" | "removed";

export function FolderManager({
  open,
  game,
  folders,
  files,
  hiddenTracks,
  isScanning,
  onClose,
  onAddFolder,
  onAddFile,
  onRemoveFolder,
  onRemoveFile,
  onRestoreHiddenTrack,
  onScan,
}: FolderManagerProps) {
  const [activeTab, setActiveTab] = useState<SourceTab>("folders");
  const [query, setQuery] = useState("");
  const fileLabel = game === "rhythmDoctor" ? "添加 RD 谱面" : "添加 ADOFAI 谱面";
  const activeSources = activeTab === "folders" ? folders : activeTab === "files" ? files : [];
  const filteredSources = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) {
      return activeSources;
    }
    return activeSources.filter((source) => source.toLowerCase().includes(normalized));
  }, [activeSources, query]);
  const filteredHiddenTracks = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) {
      return hiddenTracks;
    }
    return hiddenTracks.filter((track) =>
      [track.title, track.artist, track.author, track.chartPath, track.folderPath]
        .join(" ")
        .toLowerCase()
        .includes(normalized),
    );
  }, [hiddenTracks, query]);

  if (!open) {
    return null;
  }

  return (
    <div className="modal-layer" role="dialog" aria-modal="true" aria-label="管理本地来源">
      <div className="folder-modal">
        <header>
          <div>
            <h2>管理本地来源</h2>
            <p>添加文件夹或单个谱面，曲库会自动扫描音乐和封面。</p>
          </div>
          <button className="plain-icon" type="button" onClick={onClose} title="关闭">
            <X aria-hidden="true" />
          </button>
        </header>
        <div className="folder-actions">
          <button className="pill-button main" type="button" onClick={onAddFolder}>
            <FolderPlus aria-hidden="true" />
            <span>添加文件夹</span>
          </button>
          <button className="pill-button" type="button" onClick={onAddFile}>
            <FileMusic aria-hidden="true" />
            <span>{fileLabel}</span>
          </button>
          <button className="pill-button" type="button" onClick={onScan}>
            <RefreshCw className={isScanning ? "spinning-icon" : ""} aria-hidden="true" />
            <span>{isScanning ? "扫描中" : "重新扫描"}</span>
          </button>
        </div>

        <div className="source-tabs" role="tablist" aria-label="来源类型">
          <button
            className={activeTab === "folders" ? "active" : ""}
            type="button"
            onClick={() => setActiveTab("folders")}
          >
            文件夹 <span>{folders.length}</span>
          </button>
          <button
            className={activeTab === "files" ? "active" : ""}
            type="button"
            onClick={() => setActiveTab("files")}
          >
            单曲 <span>{files.length}</span>
          </button>
          <button
            className={activeTab === "removed" ? "active" : ""}
            type="button"
            onClick={() => setActiveTab("removed")}
          >
            已移除 <span>{hiddenTracks.length}</span>
          </button>
        </div>

        <label className="source-search">
          <Search aria-hidden="true" />
          <input
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder={searchPlaceholder(activeTab)}
          />
        </label>

        <div className="folder-list source-scroll">
          {activeTab === "removed" ? (
            filteredHiddenTracks.length === 0 ? (
              <div className="folder-empty">{emptyText(activeTab, query)}</div>
            ) : (
              filteredHiddenTracks.map((track) => (
                <div className="folder-row source-row" key={`${track.id}-${track.chartPath}`}>
                  <span title={track.chartPath || track.id}>
                    <strong>{track.title}</strong>
                    <small>{track.artist}</small>
                    <small>{track.chartPath || track.id}</small>
                  </span>
                  <button
                    className="restore-source-button"
                    type="button"
                    onClick={() => onRestoreHiddenTrack(track)}
                    title="恢复"
                  >
                    <RotateCcw aria-hidden="true" />
                  </button>
                </div>
              ))
            )
          ) : filteredSources.length === 0 ? (
            <div className="folder-empty">{emptyText(activeTab, query)}</div>
          ) : (
            filteredSources.map((source) => (
              <div className="folder-row source-row" key={source}>
                <span title={source}>
                  <strong>{sourceName(source)}</strong>
                  <small>{source}</small>
                </span>
                <button
                  type="button"
                  onClick={() => {
                    if (activeTab === "folders") {
                      onRemoveFolder(source);
                    } else {
                      onRemoveFile(source);
                    }
                  }}
                  title="移除"
                >
                  <Trash2 aria-hidden="true" />
                </button>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

function sourceName(source: string) {
  const parts = source.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? source;
}

function emptyText(activeTab: SourceTab, query: string) {
  if (query.trim()) {
    return "没有匹配的来源。";
  }
  if (activeTab === "folders") {
    return "还没有添加文件夹。";
  }
  if (activeTab === "files") {
    return "还没有添加单曲。";
  }
  return "还没有移除过曲目。";
}

function searchPlaceholder(activeTab: SourceTab) {
  if (activeTab === "folders") {
    return "搜索文件夹";
  }
  if (activeTab === "files") {
    return "搜索单曲";
  }
  return "搜索已移除曲目";
}
