import {
  Library,
  ListMusic,
  Music2,
  PanelRight,
  RefreshCw,
  Settings,
} from "lucide-react";
import type { ReactNode } from "react";
import { EmptyArtwork } from "./EmptyArtwork";
import { StatPill } from "./StatPill";
import { formatDuration } from "../lib/format";
import type { AppView, LibrarySettings, TrackSummary } from "../types/domain";

interface AppShellProps {
  activeView: AppView;
  settings: LibrarySettings | null;
  tracks: TrackSummary[];
  selectedTrack: TrackSummary | null;
  isScanning: boolean;
  children: ReactNode;
  onViewChange: (view: AppView) => void;
  onScan: () => void;
}

const navItems: Array<{ id: AppView; label: string; icon: typeof Library }> = [
  { id: "library", label: "曲库", icon: Library },
  { id: "nowPlaying", label: "正在播放", icon: Music2 },
  { id: "detail", label: "曲目详情", icon: PanelRight },
  { id: "settings", label: "设置", icon: Settings },
];

export function AppShell({
  activeView,
  settings,
  tracks,
  selectedTrack,
  isScanning,
  children,
  onViewChange,
  onScan,
}: AppShellProps) {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-block">
          <div className="brand-mark" aria-hidden="true">
            <ListMusic />
          </div>
          <div>
            <p className="eyebrow">ADOFAI</p>
            <h1>Music Box</h1>
          </div>
        </div>

        <nav className="main-nav" aria-label="主导航">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                className={item.id === activeView ? "nav-item active" : "nav-item"}
                key={item.id}
                type="button"
                onClick={() => onViewChange(item.id)}
              >
                <Icon aria-hidden="true" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="source-summary">
          <p className="section-label">来源</p>
          <strong>{settings?.folders.length ?? 0}</strong>
          <span>个文件夹</span>
        </div>

        <div className="source-summary">
          <p className="section-label">曲目</p>
          <strong>{tracks.length}</strong>
          <span>首音乐</span>
        </div>

        <button className="secondary-button full-width" type="button" onClick={onScan}>
          <RefreshCw className={isScanning ? "spin" : ""} aria-hidden="true" />
          <span>{isScanning ? "扫描中" : "重新扫描"}</span>
        </button>

        <div className="sidebar-now">
          <p className="section-label">当前</p>
          <strong>{selectedTrack?.title ?? "还没有播放"}</strong>
          <span>{selectedTrack?.artist ?? "从曲库选择音乐"}</span>
        </div>
      </aside>

      <main className="main-panel">{children}</main>

      <aside className="right-rail">
        <p className="eyebrow">Now Playing</p>
        <EmptyArtwork
          title={selectedTrack?.title ?? "未选择曲目"}
          imagePath={selectedTrack?.coverPath}
          size="md"
        />
        <div className="rail-track">
          <strong>{selectedTrack?.title ?? "未选择曲目"}</strong>
          <span>{selectedTrack?.artist ?? "ADOFAI Music Box"}</span>
        </div>
        <div className="rail-stats">
          <StatPill label="BPM" value={selectedTrack ? `${Math.round(selectedTrack.bpm)}` : "--"} />
          <StatPill label="时长" value={formatDuration(selectedTrack?.duration ?? 0)} />
        </div>
        <div className="rail-list">
          <div>
            <span>谱师</span>
            <strong>{selectedTrack?.author ?? "--"}</strong>
          </div>
          <div>
            <span>视频</span>
            <strong>{selectedTrack?.hasVideo ? "有" : "无"}</strong>
          </div>
          <div>
            <span>状态</span>
            <strong>{selectedTrack ? statusText(selectedTrack.parseStatus) : "--"}</strong>
          </div>
        </div>
      </aside>
    </div>
  );
}

function statusText(status: TrackSummary["parseStatus"]) {
  switch (status) {
    case "ok":
      return "正常";
    case "lenient":
      return "兼容";
    case "warning":
      return "留意";
    case "error":
      return "不可播放";
  }
}
