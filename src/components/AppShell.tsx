import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  FolderPlus,
  Heart,
  Home,
  Library,
  ListMusic,
  Maximize2,
  Minimize,
  Music2,
  Search,
  Settings,
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
  onAddFolder: () => void;
  onOpenFolderManager: () => void;
  onScan: () => void;
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
  onAddFolder,
  onOpenFolderManager,
  onScan,
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
        <button className="brand-card" type="button" onClick={() => onViewChange("local")}>
          <span className="brand-mark" aria-hidden="true">
            <ListMusic />
          </span>
          <span>
            <strong>ADOFAI</strong>
            <small>Music Box</small>
          </span>
          <ChevronDown aria-hidden="true" />
        </button>

        <div className="quick-grid" aria-label="快捷入口">
          <button type="button" className="quick-tile active" onClick={() => onViewChange("local")}>
            <Home aria-hidden="true" />
          </button>
          <button type="button" className="quick-tile" onClick={onOpenFolderManager}>
            <Settings aria-hidden="true" />
          </button>
          <button type="button" className="quick-tile" onClick={() => onViewChange("favorites")}>
            <Heart aria-hidden="true" />
          </button>
          <button type="button" className="quick-tile dashed" onClick={onAddFolder}>
            <FolderPlus aria-hidden="true" />
          </button>
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
            <button type="button" onClick={onOpenFolderManager} title="管理文件夹">
              <Settings aria-hidden="true" />
            </button>
          </div>
          <button className="source-card" type="button" onClick={onOpenFolderManager}>
            <strong>{settings?.folders.length ?? 0}</strong>
            <span>个文件夹</span>
          </button>
          <button className="source-card" type="button" onClick={onScan}>
            <strong>{tracks.length}</strong>
            <span>{isScanning ? "正在整理" : "首曲目"}</span>
          </button>
        </section>
      </aside>

      <section className="content-shell">
        <header
          className="titlebar"
          onMouseDown={handleTitlebarMouseDown}
          onDoubleClick={handleTitlebarDoubleClick}
        >
          <div className="history-controls" data-no-window-drag>
            <button type="button" title="后退">
              <ChevronLeft aria-hidden="true" />
            </button>
            <button type="button" title="前进">
              <ChevronRight aria-hidden="true" />
            </button>
          </div>
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
