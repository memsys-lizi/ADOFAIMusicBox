interface VolumeMixerProps {
  masterVolume: number;
  musicVolume: number;
  hitSoundVolume: number;
  playSoundVolume: number;
  onMasterVolumeChange: (volume: number) => void;
  onMusicVolumeChange: (volume: number) => void;
  onHitSoundVolumeChange: (volume: number) => void;
  onPlaySoundVolumeChange: (volume: number) => void;
}

export function VolumeMixer({
  masterVolume,
  musicVolume,
  hitSoundVolume,
  playSoundVolume,
  onMasterVolumeChange,
  onMusicVolumeChange,
  onHitSoundVolumeChange,
  onPlaySoundVolumeChange,
}: VolumeMixerProps) {
  return (
    <div className="volume-mixer">
      <VolumeSlider label="总音量" value={masterVolume} onChange={onMasterVolumeChange} />
      <VolumeSlider label="音乐" value={musicVolume} onChange={onMusicVolumeChange} />
      <VolumeSlider label="打拍音" value={hitSoundVolume} onChange={onHitSoundVolumeChange} />
      <VolumeSlider label="音效" value={playSoundVolume} onChange={onPlaySoundVolumeChange} />
    </div>
  );
}

interface VolumeSliderProps {
  label: string;
  value: number;
  onChange: (volume: number) => void;
}

function VolumeSlider({ label, value, onChange }: VolumeSliderProps) {
  return (
    <label className="mix-slider">
      <span>{label}</span>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={value}
        onChange={(event) => onChange(Number(event.currentTarget.value))}
        aria-label={`${label}音量`}
      />
      <b>{Math.round(value * 100)}%</b>
    </label>
  );
}
