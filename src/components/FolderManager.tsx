import { FolderPlus, RefreshCw, Trash2, X } from "lucide-react";

interface FolderManagerProps {
  open: boolean;
  folders: string[];
  isScanning: boolean;
  onClose: () => void;
  onAddFolder: () => void;
  onRemoveFolder: (folder: string) => void;
  onScan: () => void;
}

export function FolderManager({
  open,
  folders,
  isScanning,
  onClose,
  onAddFolder,
  onRemoveFolder,
  onScan,
}: FolderManagerProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="modal-layer" role="dialog" aria-modal="true" aria-label="管理文件夹">
      <div className="folder-modal">
        <header>
          <div>
            <h2>管理文件夹</h2>
            <p>添加或移除本地 ADOFAI 音乐来源。</p>
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
          <button className="pill-button" type="button" onClick={onScan}>
            <RefreshCw className={isScanning ? "spinning-icon" : ""} aria-hidden="true" />
            <span>{isScanning ? "整理中" : "重新整理"}</span>
          </button>
        </div>
        <div className="folder-list">
          {folders.length === 0 ? (
            <div className="folder-empty">还没有添加文件夹。</div>
          ) : (
            folders.map((folder) => (
              <div className="folder-row" key={folder}>
                <span title={folder}>{folder}</span>
                <button type="button" onClick={() => onRemoveFolder(folder)} title="移除">
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
