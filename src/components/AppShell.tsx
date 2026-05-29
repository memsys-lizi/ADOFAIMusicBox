import {
  Heart,
  Library,
  ListMusic,
  Maximize2,
  Minimize,
  Music2,
  Search,
  X,
} from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { runWindowAction, startWindowDrag } from "../lib/window";
import type { AppView, LibrarySettings, TrackSummary } from "../types/domain";

interface AppShellProps {
  activeView: AppView;
  settings: LibrarySettings | null;
  tracks: TrackSummary[];
  favoriteCount: number;
  recentCount: number;
  searchQuery: string;
  isScanning: boolean;
  children: ReactNode;
  onViewChange: (view: AppView) => void;
  onSearchChange: (query: string) => void;
  onOpenFolderManager: () => void;
}

const navItems: Array<{ id: AppView; label: string; icon: typeof Library; count: "tracks" | "favorites" | "recent" }> = [
  { id: "favorites", label: "喜欢", icon: Heart, count: "favorites" },
  { id: "recent", label: "最近播放", icon: Music2, count: "recent" },
  { id: "local", label: "本地", icon: Library, count: "tracks" },
];

export function AppShell({
  activeView,
  settings,
  tracks,
  favoriteCount,
  recentCount,
  searchQuery,
  isScanning,
  children,
  onViewChange,
  onSearchChange,
  onOpenFolderManager,
}: AppShellProps) {
  function handleTitlebarMouseDown(event: MouseEvent<HTMLElement>) {
    if (event.button !== 0 || shouldSkipWindowDrag(event.target)) {
      return;
    }
    event.preventDefault();
    startWindowDrag();
  }

  function handleTitlebarDoubleClick(event: MouseEvent<HTMLElement>) {
    if (shouldSkipWindowDrag(event.target)) {
      return;
    }
    void runWindowAction("maximize");
  }

  return (
    <div className="app-frame">
      <aside className="sidebar">
        <div className="brand-card">
          <span className="brand-mark" aria-hidden="true">
            <ListMusic />
          </span>
          <span>
            <strong>ADOFAI</strong>
            <small>Music Box</small>
          </span>
        </div>

        <nav className="main-nav" aria-label="主导航">
          {navItems.map((item) => {
            const Icon = item.icon;
            const count =
              item.count === "favorites"
                ? favoriteCount
                : item.count === "recent"
                  ? recentCount
                  : tracks.length;
            return (
              <button
                className={item.id === activeView ? "nav-item active" : "nav-item"}
                key={item.id}
                type="button"
                onClick={() => onViewChange(item.id)}
              >
                <Icon aria-hidden="true" />
                <span>{item.label}</span>
                <b>{count}</b>
              </button>
            );
          })}
        </nav>

        <section className="sidebar-section">
          <div className="section-title">
            <span>本地来源</span>
          </div>
          <button className="source-manage" type="button" onClick={onOpenFolderManager}>
            <span>管理来源</span>
            <strong>{(settings?.folders.length ?? 0) + (settings?.adofaiFiles?.length ?? 0)}</strong>
          </button>
          <div className="source-stats" aria-label="本地来源统计">
            <span>文件夹 {settings?.folders.length ?? 0}</span>
            <span>单曲 {settings?.adofaiFiles?.length ?? 0}</span>
            <span>{isScanning ? "扫描中" : `曲目 ${tracks.length}`}</span>
          </div>
        </section>
      </aside>

      <section className="content-shell">
        <header
          className="titlebar"
          onMouseDown={handleTitlebarMouseDown}
          onDoubleClick={handleTitlebarDoubleClick}
        >
          <label className="global-search" data-no-window-drag>
            <Search aria-hidden="true" />
            <input
              value={searchQuery}
              onChange={(event) => onSearchChange(event.currentTarget.value)}
              placeholder="搜索音乐"
            />
          </label>
          <div className="titlebar-spacer" />
          <div className="window-controls" data-no-window-drag>
            <button type="button" title="最小化" onClick={() => void runWindowAction("minimize")}>
              <Minimize aria-hidden="true" />
            </button>
            <button type="button" title="最大化" onClick={() => void runWindowAction("maximize")}>
              <Maximize2 aria-hidden="true" />
            </button>
            <button type="button" title="关闭" onClick={() => void runWindowAction("close")}>
              <X aria-hidden="true" />
            </button>
          </div>
        </header>
        <main className="main-panel">{children}</main>
      </section>
    </div>
  );
}

function shouldSkipWindowDrag(target: EventTarget) {
  return target instanceof Element && Boolean(target.closest("[data-no-window-drag]"));
}
