import { useCallback, useEffect, useRef, useState } from "react";
import { toAssetUrl } from "../lib/assets";
import type { AudioTimeline, HitEvent, TrackSummary } from "../types/domain";

interface UseAdoAudioOptions {
  musicVolume: number;
  hitSoundVolume: number;
  playSoundVolume: number;
  onEnded?: () => void;
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
const LOOKAHEAD_SECONDS = 0.16;
const LATE_TOLERANCE_SECONDS = 0.025;
const SCHEDULER_INTERVAL_MS = 25;

export function useAdoAudio(options: UseAdoAudioOptions): AdoAudioApi {
  const contextRef = useRef<AudioContext | null>(null);
  const mainGainRef = useRef<GainNode | null>(null);
  const sourceRef = useRef<AudioBufferSourceNode | null>(null);
  const mainBufferRef = useRef<AudioBuffer | null>(null);
  const timelineRef = useRef<AudioTimeline | null>(null);
  const mainBuffersRef = useRef<Map<string, AudioBuffer>>(new Map());
  const soundBuffersRef = useRef<Map<string, AudioBuffer>>(new Map());
  const scheduledRef = useRef<Set<string>>(new Set());
  const intervalRef = useRef<number | null>(null);
  const frameRef = useRef<number | null>(null);
  const positionRef = useRef(0);
  const startedAtRef = useRef(0);
  const durationRef = useRef(0);
  const playingRef = useRef(false);
  const sessionRef = useRef(0);
  const onEndedRef = useRef<(() => void) | undefined>(options.onEnded);

  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [hitSoundsEnabled, setHitSoundsEnabled] = useState(true);
  const [playSoundsEnabled, setPlaySoundsEnabled] = useState(true);

  const ensureContext = useCallback(() => {
    if (!contextRef.current) {
      contextRef.current = new AudioContext();
    }
    return contextRef.current;
  }, []);

  const ensureMainGain = useCallback(() => {
    const context = ensureContext();
    if (!mainGainRef.current) {
      mainGainRef.current = context.createGain();
      mainGainRef.current.gain.value = clampVolume(options.musicVolume);
      mainGainRef.current.connect(context.destination);
    }
    return mainGainRef.current;
  }, [ensureContext, options.musicVolume]);

  const transportTime = useCallback(() => {
    const context = contextRef.current;
    if (playingRef.current && context) {
      return Math.min(
        durationRef.current,
        Math.max(0, context.currentTime - startedAtRef.current),
      );
    }
    return Math.min(durationRef.current, Math.max(0, positionRef.current));
  }, []);

  const stopScheduler = useCallback(() => {
    if (intervalRef.current !== null) {
      window.clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  const stopSource = useCallback(() => {
    const source = sourceRef.current;
    if (!source) {
      return;
    }
    source.onended = null;
    try {
      source.stop();
    } catch {
      // The source may already have ended.
    }
    source.disconnect();
    sourceRef.current = null;
  }, []);

  const decodeUrl = useCallback(
    async (url: string, cache: Map<string, AudioBuffer>, cacheKey = url) => {
      const existing = cache.get(cacheKey);
      if (existing) {
        return existing;
      }
      const context = ensureContext();
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error("音频文件无法读取");
      }
      const bytes = await response.arrayBuffer();
      const buffer = await context.decodeAudioData(bytes.slice(0));
      cache.set(cacheKey, buffer);
      return buffer;
    },
    [ensureContext],
  );

  const decodeSound = useCallback(
    async (soundName: string): Promise<AudioBuffer | null> => {
      const existing = soundBuffersRef.current.get(soundName);
      if (existing) {
        return existing;
      }

      const candidates =
        soundName.includes("\\") || soundName.includes("/")
          ? [toAssetUrl(soundName)].filter((value): value is string => Boolean(value))
          : BUILTIN_EXTENSIONS.map((ext) => `/audio/${soundName}${ext}`);

      for (const url of candidates) {
        try {
          return await decodeUrl(url, soundBuffersRef.current, soundName);
        } catch {
          continue;
        }
      }
      return null;
    },
    [decodeUrl],
  );

  const preloadTimelineSounds = useCallback(
    async (timeline: AudioTimeline) => {
      const soundNames = new Set<string>();
      for (const event of [
        ...timeline.hitEvents,
        ...timeline.holdSoundEvents,
        ...timeline.playSoundEvents,
      ]) {
        soundNames.add(event.soundName);
      }
      await Promise.all([...soundNames].map((soundName) => decodeSound(soundName)));
    },
    [decodeSound],
  );

  const scheduleEvent = useCallback(
    async (event: HitEvent, multiplier: number, namespace: string, now: number) => {
      const context = ensureContext();
      const session = sessionRef.current;
      const key = `${session}:${namespace}:${event.sourceFloor}:${event.kind}:${event.timeSec}`;
      if (scheduledRef.current.has(key)) {
        return;
      }

      const isLoop =
        event.kind === "hold-loop" &&
        typeof event.endTimeSec === "number" &&
        event.endTimeSec > event.timeSec;
      const delta = event.timeSec - now;
      const targetTime = startedAtRef.current + event.timeSec;

      if (isLoop) {
        if ((event.endTimeSec ?? 0) <= now || delta > LOOKAHEAD_SECONDS) {
          return;
        }
      } else if (delta < -LATE_TOLERANCE_SECONDS || delta > LOOKAHEAD_SECONDS) {
        return;
      }

      const buffer = await decodeSound(event.soundName);
      if (!buffer || session !== sessionRef.current) {
        scheduledRef.current.add(key);
        return;
      }

      const when = Math.max(context.currentTime, targetTime);
      const source = context.createBufferSource();
      const gain = context.createGain();
      source.buffer = buffer;
      source.playbackRate.value = Math.max(0.05, event.pitch);
      gain.gain.value = Math.max(0, event.volume * multiplier);
      source.connect(gain);
      gain.connect(context.destination);

      if (isLoop) {
        const elapsed = Math.max(0, now - event.timeSec) * Math.max(0.05, event.pitch);
        const offset = buffer.duration > 0 ? elapsed % buffer.duration : 0;
        const stopAt = Math.max(
          context.currentTime,
          startedAtRef.current + (event.endTimeSec ?? now),
        );
        source.loop = true;
        source.start(when, offset);
        source.stop(stopAt);
      } else {
        source.start(when);
        if (
          typeof event.endTimeSec === "number" &&
          event.endTimeSec > event.timeSec &&
          event.endTimeSec > now
        ) {
          source.stop(Math.max(context.currentTime, startedAtRef.current + event.endTimeSec));
        }
      }

      scheduledRef.current.add(key);
    },
    [decodeSound, ensureContext],
  );

  const runScheduler = useCallback(() => {
    const timeline = timelineRef.current;
    if (!timeline || !playingRef.current) {
      return;
    }
    const now = transportTime();
    if (hitSoundsEnabled) {
      for (const event of [...timeline.hitEvents, ...timeline.holdSoundEvents]) {
        void scheduleEvent(event, options.hitSoundVolume, "hit", now);
      }
    }
    if (playSoundsEnabled) {
      for (const event of timeline.playSoundEvents) {
        void scheduleEvent(event, options.playSoundVolume, "play", now);
      }
    }
  }, [
    hitSoundsEnabled,
    options.hitSoundVolume,
    options.playSoundVolume,
    playSoundsEnabled,
    scheduleEvent,
    transportTime,
  ]);

  const startScheduler = useCallback(() => {
    stopScheduler();
    runScheduler();
    intervalRef.current = window.setInterval(runScheduler, SCHEDULER_INTERVAL_MS);
  }, [runScheduler, stopScheduler]);

  const finishPlayback = useCallback((notify = true) => {
    stopScheduler();
    sourceRef.current = null;
    positionRef.current = durationRef.current;
    playingRef.current = false;
    setCurrentTime(durationRef.current);
    setIsPlaying(false);
    if (notify) {
      onEndedRef.current?.();
    }
  }, [stopScheduler]);

  const startSource = useCallback(
    (position: number) => {
      const context = ensureContext();
      const buffer = mainBufferRef.current;
      const timeline = timelineRef.current;
      if (!buffer || !timeline) {
        throw new Error("当前曲目还没有加载");
      }

      stopSource();
      const pitch = Math.max(0.05, timeline.pitch);
      const clampedPosition = Math.min(durationRef.current, Math.max(0, position));
      const rawOffset = Math.min(buffer.duration, clampedPosition * pitch);
      if (rawOffset >= buffer.duration) {
        finishPlayback();
        return;
      }

      sessionRef.current += 1;
      scheduledRef.current.clear();
      positionRef.current = clampedPosition;
      startedAtRef.current = context.currentTime - clampedPosition;

      const source = context.createBufferSource();
      source.buffer = buffer;
      source.playbackRate.value = pitch;
      source.connect(ensureMainGain());
      source.onended = () => {
        if (sourceRef.current === source) {
          finishPlayback();
        }
      };
      source.start(context.currentTime, rawOffset);
      sourceRef.current = source;
      playingRef.current = true;
      setIsPlaying(true);
      startScheduler();
    },
    [ensureContext, ensureMainGain, finishPlayback, startScheduler, stopSource],
  );

  const load = useCallback(
    async (track: TrackSummary, timeline: AudioTimeline) => {
      const source = toAssetUrl(track.audioPath);
      if (!source) {
        throw new Error("当前曲目没有可播放的音乐文件");
      }

      stopScheduler();
      stopSource();
      playingRef.current = false;
      setIsPlaying(false);
      sessionRef.current += 1;
      scheduledRef.current.clear();

      const buffer = await decodeUrl(source, mainBuffersRef.current, track.audioPath ?? source);
      mainBufferRef.current = buffer;
      timelineRef.current = timeline;
      positionRef.current = 0;
      startedAtRef.current = 0;
      durationRef.current = Math.max(timeline.duration, buffer.duration / Math.max(0.05, timeline.pitch));
      setCurrentTime(0);
      setDuration(durationRef.current);
      await preloadTimelineSounds(timeline);
    },
    [decodeUrl, preloadTimelineSounds, stopScheduler, stopSource],
  );

  const play = useCallback(async () => {
    const context = ensureContext();
    await context.resume();
    if (playingRef.current) {
      return;
    }
    startSource(positionRef.current);
  }, [ensureContext, startSource]);

  const pause = useCallback(() => {
    if (!playingRef.current) {
      return;
    }
    positionRef.current = transportTime();
    playingRef.current = false;
    setIsPlaying(false);
    stopScheduler();
    stopSource();
  }, [stopScheduler, stopSource, transportTime]);

  const seek = useCallback(
    (time: number) => {
      const wasPlaying = playingRef.current;
      const nextTime = Math.min(durationRef.current, Math.max(0, time));
      positionRef.current = nextTime;
      scheduledRef.current.clear();
      setCurrentTime(nextTime);
      if (wasPlaying) {
        startSource(nextTime);
      }
    },
    [startSource],
  );

  const syncClock = useCallback(() => {
    const nextTime = transportTime();
    setCurrentTime(nextTime);
    setDuration(durationRef.current);
    frameRef.current = window.requestAnimationFrame(syncClock);
  }, [transportTime]);

  useEffect(() => {
    onEndedRef.current = options.onEnded;
  }, [options.onEnded]);

  useEffect(() => {
    const context = contextRef.current;
    const gain = mainGainRef.current;
    if (context && gain) {
      gain.gain.setTargetAtTime(clampVolume(options.musicVolume), context.currentTime, 0.01);
    }
  }, [options.musicVolume]);

  useEffect(() => {
    frameRef.current = window.requestAnimationFrame(syncClock);
    return () => {
      stopScheduler();
      stopSource();
      if (frameRef.current !== null) {
        window.cancelAnimationFrame(frameRef.current);
      }
    };
  }, [stopScheduler, stopSource, syncClock]);

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

function clampVolume(value: number) {
  return Math.min(1, Math.max(0, value));
}
