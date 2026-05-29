import { useCallback, useEffect, useRef, useState } from "react";
import { toAssetUrl } from "../lib/assets";
import type { AudioTimeline, HitEvent, TrackSummary } from "../types/domain";

interface UseAdoAudioOptions {
  hitSoundVolume: number;
  playSoundVolume: number;
}

interface AdoAudioApi {
  currentTime: number;
  duration: number;
  isPlaying: boolean;
  load: (track: TrackSummary, timeline: AudioTimeline) => Promise<void>;
  play: () => Promise<void>;
  pause: () => void;
  seek: (time: number) => void;
  setHitSoundsEnabled: (enabled: boolean) => void;
  setPlaySoundsEnabled: (enabled: boolean) => void;
  hitSoundsEnabled: boolean;
  playSoundsEnabled: boolean;
}

const BUILTIN_EXTENSIONS = [".wav", ".ogg", ".mp3"];
const LOOKAHEAD_SECONDS = 0.22;
const SCHEDULER_INTERVAL_MS = 60;

export function useAdoAudio(options: UseAdoAudioOptions): AdoAudioApi {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const contextRef = useRef<AudioContext | null>(null);
  const timelineRef = useRef<AudioTimeline | null>(null);
  const scheduledRef = useRef<Set<string>>(new Set());
  const buffersRef = useRef<Map<string, AudioBuffer>>(new Map());
  const intervalRef = useRef<number | null>(null);
  const frameRef = useRef<number | null>(null);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [hitSoundsEnabled, setHitSoundsEnabled] = useState(true);
  const [playSoundsEnabled, setPlaySoundsEnabled] = useState(true);

  const ensureAudio = useCallback(() => {
    if (!audioRef.current) {
      audioRef.current = new Audio();
      audioRef.current.preload = "auto";
    }
    return audioRef.current;
  }, []);

  const ensureContext = useCallback(() => {
    if (!contextRef.current) {
      contextRef.current = new AudioContext();
    }
    return contextRef.current;
  }, []);

  const decodeSound = useCallback(
    async (soundName: string): Promise<AudioBuffer | null> => {
      const existing = buffersRef.current.get(soundName);
      if (existing) {
        return existing;
      }

      const candidates = soundName.includes("\\") || soundName.includes("/")
        ? [toAssetUrl(soundName)].filter((value): value is string => Boolean(value))
        : BUILTIN_EXTENSIONS.map((ext) => `/audio/${soundName}${ext}`);

      const context = ensureContext();
      for (const url of candidates) {
        try {
          const response = await fetch(url);
          if (!response.ok) {
            continue;
          }
          const data = await response.arrayBuffer();
          const buffer = await context.decodeAudioData(data.slice(0));
          buffersRef.current.set(soundName, buffer);
          return buffer;
        } catch {
          continue;
        }
      }
      return null;
    },
    [ensureContext],
  );

  const scheduleEvent = useCallback(
    async (event: HitEvent, multiplier: number, namespace: string) => {
      const audio = ensureAudio();
      const context = ensureContext();
      const key = `${namespace}:${event.sourceFloor}:${event.kind}:${event.timeSec}`;
      if (scheduledRef.current.has(key)) {
        return;
      }
      const delta = event.timeSec - audio.currentTime;
      if (delta < -0.05 || delta > LOOKAHEAD_SECONDS) {
        return;
      }
      const buffer = await decodeSound(event.soundName);
      if (!buffer) {
        scheduledRef.current.add(key);
        return;
      }
      const source = context.createBufferSource();
      const gain = context.createGain();
      source.buffer = buffer;
      source.playbackRate.value = Math.max(0.05, event.pitch);
      gain.gain.value = Math.max(0, event.volume * multiplier);
      source.connect(gain);
      gain.connect(context.destination);
      source.start(context.currentTime + Math.max(0, delta));
      scheduledRef.current.add(key);
    },
    [decodeSound, ensureAudio, ensureContext],
  );

  const runScheduler = useCallback(() => {
    const timeline = timelineRef.current;
    if (!timeline) {
      return;
    }
    const hitEvents = hitSoundsEnabled
      ? [...timeline.hitEvents, ...timeline.holdSoundEvents]
      : [];
    for (const event of hitEvents) {
      void scheduleEvent(event, options.hitSoundVolume, "hit");
    }
    if (playSoundsEnabled) {
      for (const event of timeline.playSoundEvents) {
        void scheduleEvent(event, options.playSoundVolume, "play");
      }
    }
  }, [
    hitSoundsEnabled,
    options.hitSoundVolume,
    options.playSoundVolume,
    playSoundsEnabled,
    scheduleEvent,
  ]);

  const stopScheduler = useCallback(() => {
    if (intervalRef.current !== null) {
      window.clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  const startScheduler = useCallback(() => {
    stopScheduler();
    runScheduler();
    intervalRef.current = window.setInterval(runScheduler, SCHEDULER_INTERVAL_MS);
  }, [runScheduler, stopScheduler]);

  const syncClock = useCallback(() => {
    const audio = audioRef.current;
    if (audio) {
      setCurrentTime(audio.currentTime);
      setDuration(Number.isFinite(audio.duration) ? audio.duration : timelineRef.current?.duration ?? 0);
    }
    frameRef.current = window.requestAnimationFrame(syncClock);
  }, []);

  const load = useCallback(
    async (track: TrackSummary, timeline: AudioTimeline) => {
      const audio = ensureAudio();
      const source = toAssetUrl(track.audioPath);
      if (!source) {
        throw new Error("当前谱面没有可播放的音乐文件");
      }
      stopScheduler();
      pauseAudio(audio);
      setIsPlaying(false);
      scheduledRef.current.clear();
      timelineRef.current = timeline;
      audio.src = source;
      audio.volume = 1;
      audio.playbackRate = Math.max(0.05, timeline.pitch);
      audio.currentTime = 0;
      setCurrentTime(0);
      setDuration(timeline.duration);
      await loadMedia(audio);
    },
    [ensureAudio, stopScheduler],
  );

  const play = useCallback(async () => {
    const audio = ensureAudio();
    const context = ensureContext();
    await context.resume();
    await audio.play();
    setIsPlaying(true);
    startScheduler();
  }, [ensureAudio, ensureContext, startScheduler]);

  const pause = useCallback(() => {
    const audio = ensureAudio();
    pauseAudio(audio);
    setIsPlaying(false);
    stopScheduler();
  }, [ensureAudio, stopScheduler]);

  const seek = useCallback((time: number) => {
    const audio = ensureAudio();
    audio.currentTime = Math.max(0, time);
    scheduledRef.current.clear();
    setCurrentTime(audio.currentTime);
  }, [ensureAudio]);

  useEffect(() => {
    const audio = ensureAudio();
    const ended = () => {
      setIsPlaying(false);
      stopScheduler();
    };
    audio.addEventListener("ended", ended);
    frameRef.current = window.requestAnimationFrame(syncClock);
    return () => {
      audio.removeEventListener("ended", ended);
      stopScheduler();
      if (frameRef.current !== null) {
        window.cancelAnimationFrame(frameRef.current);
      }
      pauseAudio(audio);
    };
  }, [ensureAudio, stopScheduler, syncClock]);

  return {
    currentTime,
    duration,
    isPlaying,
    load,
    play,
    pause,
    seek,
    setHitSoundsEnabled,
    setPlaySoundsEnabled,
    hitSoundsEnabled,
    playSoundsEnabled,
  };
}

function pauseAudio(audio: HTMLAudioElement) {
  if (!audio.paused) {
    audio.pause();
  }
}

function loadMedia(audio: HTMLAudioElement) {
  return new Promise<void>((resolve, reject) => {
    const cleanup = () => {
      audio.removeEventListener("loadedmetadata", onReady);
      audio.removeEventListener("canplay", onReady);
      audio.removeEventListener("error", onError);
    };
    const onReady = () => {
      cleanup();
      resolve();
    };
    const onError = () => {
      cleanup();
      reject(new Error("音乐文件无法播放"));
    };
    audio.addEventListener("loadedmetadata", onReady, { once: true });
    audio.addEventListener("canplay", onReady, { once: true });
    audio.addEventListener("error", onError, { once: true });
    audio.load();
  });
}
