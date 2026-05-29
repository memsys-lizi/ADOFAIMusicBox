import { AlertTriangle, CheckCircle2, FileJson, FolderOpen, Music2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { EmptyArtwork } from "../../components/EmptyArtwork";
import { StatPill } from "../../components/StatPill";
import { basename, formatCount, formatDuration } from "../../lib/format";
import { getTrackDetail } from "../../lib/tauri";
import type { TrackDetail, TrackSummary } from "../../types/domain";

interface LevelDetailViewProps {
  track: TrackSummary | null;
}

export function LevelDetailView({ track }: LevelDetailViewProps) {
  const [detail, setDetail] = useState<TrackDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setDetail(null);
    setError(null);
    if (!track) {
      return;
    }
    getTrackDetail(track.adofaiPath)
      .then((next) => {
        if (!cancelled) {
          setDetail(next);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [track]);

  const topEvents = useMemo(() => {
    if (!detail) {
      return [];
    }
    return Object.entries(detail.eventCounts)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 12);
  }, [detail]);

  if (!track) {
    return (
      <section className="page">
        <div className="empty-state compact">
          <FileJson aria-hidden="true" />
          <h3>选择一张谱面</h3>
        </div>
      </section>
    );
  }

  return (
    <section className="page detail-page">
      <header className="detail-hero">
        <EmptyArtwork title={track.title} imagePath={track.coverPath} size="lg" />
        <div>
          <p className="eyebrow">Level Detail</p>
          <h2>{track.title}</h2>
          <p>{track.artist} · 谱师 {track.author}</p>
          <div className="library-stats inline">
            <StatPill label="BPM" value={`${Math.round(track.bpm)}`} />
            <StatPill label="时长" value={formatDuration(track.duration)} />
            <StatPill label="状态" value={statusText(track.parseStatus)} />
          </div>
        </div>
      </header>

      {error && (
        <div className="notice error">
          <AlertTriangle aria-hidden="true" />
          <span>{error}</span>
        </div>
      )}

      {detail && (
        <div className="detail-grid">
          <div className="settings-panel">
            <div className="panel-title">
              <Music2 aria-hidden="true" />
              <strong>文件</strong>
            </div>
            <ResourceRow label="歌曲文件" ok={detail.resourceStatus.audioExists} value={basename(track.audioPath ?? "")} />
            <ResourceRow label="封面" ok={detail.resourceStatus.coverExists} value={basename(track.coverPath ?? "")} />
            <ResourceRow label="列表图标" ok={detail.resourceStatus.iconExists} value={basename(track.iconPath ?? "")} />
            <ResourceRow label="视频" ok={detail.resourceStatus.videoExists} value={basename(track.videoPath ?? "")} />
          </div>

          <div className="settings-panel">
            <div className="panel-title">
              <FileJson aria-hidden="true" />
              <strong>音乐内容</strong>
            </div>
            <ResourceRow label="读取方式" ok value={readModeLabel(detail.rawParseMode)} />
            <ResourceRow label="谱面内容" ok value={`${formatCount(Object.values(detail.eventCounts).reduce((a, b) => a + b, 0))} 项`} />
            <ResourceRow label="可播放音效" ok value={audioFeatureText(detail.supportedAudioEvents)} />
          </div>

          <div className="settings-panel wide">
            <div className="panel-title">
              <FolderOpen aria-hidden="true" />
              <strong>文件位置</strong>
            </div>
            <code className="folder-path">{track.adofaiPath}</code>
            <code className="folder-path">{track.folderPath}</code>
          </div>

          <div className="settings-panel wide">
            <div className="panel-title">
              <CheckCircle2 aria-hidden="true" />
              <strong>谱面内容</strong>
            </div>
            <div className="event-cloud">
              {topEvents.map(([name, count]) => (
                <span key={name}>
                  {name}
                  <b>{count}</b>
                </span>
              ))}
            </div>
          </div>

          {detail.warnings.length > 0 && (
            <div className="settings-panel wide">
              <div className="panel-title">
                <AlertTriangle aria-hidden="true" />
                <strong>需要留意</strong>
              </div>
              {detail.warnings.map((warning) => (
                <p className="warning-line" key={warning}>
                  {warning}
                </p>
              ))}
            </div>
          )}
        </div>
      )}
    </section>
  );
}

interface ResourceRowProps {
  label: string;
  ok: boolean;
  value: string;
}

function ResourceRow({ label, ok, value }: ResourceRowProps) {
  return (
    <div className="resource-row">
      {ok ? <CheckCircle2 aria-hidden="true" /> : <AlertTriangle aria-hidden="true" />}
      <span>{label}</span>
      <strong>{value || "未找到"}</strong>
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

function readModeLabel(mode: string) {
  if (mode === "strict") {
    return "正常读取";
  }
  if (mode === "lenient") {
    return "兼容读取";
  }
  return mode || "--";
}

function audioFeatureText(events: string[]) {
  const names = events.map((name) => {
    switch (name) {
      case "SetHitsound":
        return "打拍音";
      case "PlaySound":
        return "谱面音效";
      case "Hold":
      case "SetHoldSound":
        return "长按音";
      case "FreeRoam":
        return "自由移动音";
      case "MultiPlanet":
      case "Multitap":
        return "特殊节拍";
      default:
        return name;
    }
  });
  return Array.from(new Set(names)).join(", ") || "无";
}
