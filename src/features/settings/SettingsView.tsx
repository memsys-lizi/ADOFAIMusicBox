import { Moon, Save, Sliders, Sun, Volume2 } from "lucide-react";
import type { LibrarySettings, ThemeMode } from "../../types/domain";

interface SettingsViewProps {
  settings: LibrarySettings | null;
  onSave: (settings: LibrarySettings) => Promise<void>;
}

export function SettingsView({ settings, onSave }: SettingsViewProps) {
  if (!settings) {
    return <section className="page" />;
  }

  function update(next: Partial<LibrarySettings>) {
    if (settings) {
      void onSave({ ...settings, ...next });
    }
  }

  return (
    <section className="page settings-page">
      <header className="page-header">
        <div>
          <p className="eyebrow">Settings</p>
          <h2>设置</h2>
        </div>
        <button className="secondary-button" type="button" onClick={() => void onSave(settings)}>
          <Save aria-hidden="true" />
          <span>保存</span>
        </button>
      </header>

      <div className="settings-grid">
        <div className="settings-panel">
          <div className="panel-title">
            <Sliders aria-hidden="true" />
            <strong>播放</strong>
          </div>
          <label className="control-row">
            <span>打拍音音量</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={settings.hitSoundVolume}
              onChange={(event) => update({ hitSoundVolume: Number(event.currentTarget.value) })}
            />
            <b>{Math.round(settings.hitSoundVolume * 100)}%</b>
          </label>
          <label className="control-row">
            <span>谱面音效音量</span>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={settings.playSoundVolume}
              onChange={(event) => update({ playSoundVolume: Number(event.currentTarget.value) })}
            />
            <b>{Math.round(settings.playSoundVolume * 100)}%</b>
          </label>
        </div>

        <div className="settings-panel">
          <div className="panel-title">
            <Volume2 aria-hidden="true" />
            <strong>曲库</strong>
          </div>
          <label className="switch-row">
            <span>兼容非标准谱面文件</span>
            <input
              type="checkbox"
              checked={settings.lenientParsing}
              onChange={(event) => update({ lenientParsing: event.currentTarget.checked })}
            />
          </label>
          <label className="switch-row">
            <span>缺少封面时自动生成</span>
            <input
              type="checkbox"
              checked={settings.defaultCoverMode === "generated"}
              onChange={(event) =>
                update({ defaultCoverMode: event.currentTarget.checked ? "generated" : "minimal" })
              }
            />
          </label>
        </div>

        <div className="settings-panel">
          <div className="panel-title">
            {settings.theme === "light" ? <Sun aria-hidden="true" /> : <Moon aria-hidden="true" />}
            <strong>主题</strong>
          </div>
          <div className="theme-options">
            {(["dark", "light", "system"] satisfies ThemeMode[]).map((theme) => (
              <button
                key={theme}
                className={settings.theme === theme ? "theme-card active" : "theme-card"}
                type="button"
                onClick={() => update({ theme })}
              >
                <span>{themeName(theme)}</span>
              </button>
            ))}
          </div>
        </div>

        <div className="settings-panel folder-panel">
          <div className="panel-title">
            <Sliders aria-hidden="true" />
            <strong>曲库来源</strong>
          </div>
          {settings.folders.length === 0 ? (
            <span className="muted">暂无来源</span>
          ) : (
            settings.folders.map((folder) => (
              <code className="folder-path" key={folder}>
                {folder}
              </code>
            ))
          )}
        </div>
      </div>
    </section>
  );
}

function themeName(theme: ThemeMode) {
  switch (theme) {
    case "dark":
      return "深色";
    case "light":
      return "浅色";
    case "system":
      return "跟随系统";
  }
}
