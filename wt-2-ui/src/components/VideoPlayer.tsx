import { useEffect, useRef, useState, useCallback } from 'react'
import {
  Play,
  Pause,
  Maximize,
  Minimize,
  PictureInPicture2,
  Headphones,
  MonitorSpeaker,
  Settings2,
  Gauge,
  AlertCircle,
  RotateCcw,
} from 'lucide-react'
import type { Format, SponsorSegment, VideoDetail } from '../lib/types.ts'
import { usePlayerStore } from '../stores/player.ts'
import { ScrubBar } from './ScrubBar.tsx'
import { QualitySpeedMenu } from './QualitySpeedMenu.tsx'

interface VideoPlayerProps {
  video: VideoDetail
  streams: { formats: Format[]; adaptiveFormats: Format[] }
  segments: SponsorSegment[]
}

export function VideoPlayer({ video, streams, segments }: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const [showControls, setShowControls] = useState(true)
  const [showQuality, setShowQuality] = useState(false)
  const [showSpeed, setShowSpeed] = useState(false)
  const [duration, setDuration] = useState(0)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const [isBuffering, setIsBuffering] = useState(false)
  const [loadError, setLoadError] = useState<string | null>(null)

  const {
    isPlaying,
    isAudioOnly,
    backgroundAudio,
    playbackRate,
    volume,
    currentTime,
    selectedQuality,
    togglePlay,
    setPlaying,
    setCurrentTime,
    setAudioOnly,
    setBackgroundAudio,
    setPlaybackRate,
    setVolume,
    setSelectedQuality,
  } = usePlayerStore()

  const allFormats = [...streams.formats, ...streams.adaptiveFormats]
  const activeFormat =
    allFormats.find((f) => f.qualityLabel === selectedQuality) ??
    streams.formats.find((f) => !f.audioOnly) ??
    allFormats[0]

  const audioFormat = streams.adaptiveFormats.find((f) => f.audioOnly)
  const isAudioSelected = selectedQuality === 'audio'
  const currentSrc = isAudioSelected ? audioFormat?.url : activeFormat?.url

  useEffect(() => {
    const el = videoRef.current
    if (!el) return
    el.playbackRate = playbackRate
  }, [playbackRate])

  useEffect(() => {
    const el = videoRef.current
    if (!el) return
    el.volume = volume
  }, [volume])

  useEffect(() => {
    const el = videoRef.current
    if (!el) return
    if (isPlaying) {
      const playPromise = el.play()
      if (playPromise !== undefined) {
        void playPromise.catch((err: Error) => {
          // Autoplay may be blocked on mobile until the user interacts.
          // We keep isPlaying true so the next user gesture resumes playback.
          if ((err as unknown as { name?: string }).name !== 'AbortError') {
            console.warn('playback failed', err)
          }
        })
      }
    } else {
      el.pause()
    }
  }, [isPlaying, currentSrc])

  useEffect(() => {
    const el = videoRef.current
    if (!el || Math.abs(el.currentTime - currentTime) < 0.5) return
    el.currentTime = currentTime
  }, [currentTime])

  useEffect(() => {
    const el = videoRef.current
    if (!el) return
    const onTimeUpdate = () => setCurrentTime(el.currentTime)
    const onLoaded = () => {
      setDuration(el.duration || video.durationSeconds)
      setLoadError(null)
    }
    const onEnded = () => setPlaying(false)
    const onWaiting = () => setIsBuffering(true)
    const onCanPlay = () => setIsBuffering(false)
    const onPlaying = () => setIsBuffering(false)
    const onError = () => {
      const code = el.error?.code ?? 0
      const messages: Record<number, string> = {
        1: 'Playback aborted',
        2: 'Network error',
        3: 'Decode error',
        4: 'Format not supported',
      }
      setLoadError(messages[code] ?? `Video error (${code})`)
      setPlaying(false)
    }
    el.addEventListener('timeupdate', onTimeUpdate)
    el.addEventListener('loadedmetadata', onLoaded)
    el.addEventListener('ended', onEnded)
    el.addEventListener('waiting', onWaiting)
    el.addEventListener('canplay', onCanPlay)
    el.addEventListener('playing', onPlaying)
    el.addEventListener('error', onError)
    return () => {
      el.removeEventListener('timeupdate', onTimeUpdate)
      el.removeEventListener('loadedmetadata', onLoaded)
      el.removeEventListener('ended', onEnded)
      el.removeEventListener('waiting', onWaiting)
      el.removeEventListener('canplay', onCanPlay)
      el.removeEventListener('playing', onPlaying)
      el.removeEventListener('error', onError)
    }
  }, [setCurrentTime, setPlaying, video.durationSeconds])

  useEffect(() => {
    const timer = setTimeout(() => setShowControls(false), 3000)
    return () => clearTimeout(timer)
  }, [isPlaying, showControls])

  const handleSeek = useCallback(
    (time: number) => {
      const el = videoRef.current
      if (el) el.currentTime = time
      setCurrentTime(time)
    },
    [setCurrentTime],
  )

  const toggleFullscreen = useCallback(async () => {
    const el = containerRef.current
    if (!el) return
    try {
      if (!document.fullscreenElement) {
        await el.requestFullscreen()
        setIsFullscreen(true)
      } else {
        await document.exitFullscreen()
        setIsFullscreen(false)
      }
    } catch {
      // ignore
    }
  }, [])

  const togglePiP = useCallback(async () => {
    const el = videoRef.current
    if (!el || !document.pictureInPictureEnabled) return
    try {
      if (document.pictureInPictureElement === el) {
        await document.exitPictureInPicture()
      } else {
        await el.requestPictureInPicture()
      }
    } catch {
      // ignore
    }
  }, [])

  const retryLoad = useCallback(() => {
    const el = videoRef.current
    if (!el || !currentSrc) return
    el.load()
    setLoadError(null)
    if (isPlaying) {
      void el.play()
    }
  }, [currentSrc, isPlaying])

  const speedItems = ['0.5', '0.75', '1', '1.25', '1.5', '2'].map((v) => ({ label: `${v}x`, value: v }))
  const qualityItems = allFormats
    .filter((f) => !f.audioOnly)
    .map((f) => ({ label: f.qualityLabel, value: f.qualityLabel }))
  if (streams.adaptiveFormats.some((f) => f.audioOnly)) {
    qualityItems.push({ label: 'Audio only', value: 'audio' })
  }

  return (
    <div
      ref={containerRef}
      className="group relative aspect-video w-full bg-black"
      onMouseMove={() => setShowControls(true)}
      onClick={() => togglePlay()}
      onTouchStart={() => setShowControls(true)}
    >
      <video
        key={currentSrc ?? 'no-src'}
        ref={videoRef}
        src={currentSrc}
        poster={video.thumbnails[0]?.url}
        className={`h-full w-full ${isAudioOnly || isAudioSelected ? 'opacity-0' : 'opacity-100'}`}
        playsInline
        preload="metadata"
        loop={false}
        muted={false}
        onClick={(e) => e.stopPropagation()}
      />
      {(isAudioOnly || isAudioSelected) && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-bg text-text">
          <Headphones className="h-16 w-16 text-accent" aria-hidden="true" />
          <p className="mt-2 text-sm font-medium">Audio-only mode</p>
        </div>
      )}

      {isBuffering && !loadError && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="h-10 w-10 animate-spin rounded-full border-4 border-white/20 border-t-accent" />
        </div>
      )}

      {loadError && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/80 px-6 text-center">
          <AlertCircle className="h-12 w-12 text-accent" aria-hidden="true" />
          <p className="mt-2 text-sm font-medium text-white">{loadError}</p>
          {currentSrc ? (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation()
                retryLoad()
              }}
              className="mt-3 flex items-center gap-2 rounded-full bg-accent px-4 py-2 text-sm font-semibold text-white"
            >
              <RotateCcw className="h-4 w-4" />
              Retry
            </button>
          ) : (
            <p className="mt-2 text-xs text-white/70">No playable stream available</p>
          )}
        </div>
      )}

      {!currentSrc && !loadError && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-black/60 px-6 text-center">
          <AlertCircle className="h-12 w-12 text-white/60" aria-hidden="true" />
          <p className="mt-2 text-sm font-medium text-white">No stream selected</p>
        </div>
      )}

      <div
        className={`absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 to-transparent px-3 pb-3 pt-8 transition-opacity ${
          showControls || !isPlaying ? 'opacity-100' : 'opacity-0'
        }`}
      >
        <div onClick={(e) => e.stopPropagation()}>
          <ScrubBar
            currentTime={currentTime}
            duration={duration || video.durationSeconds}
            segments={segments}
            chapters={video.chapters}
            onSeek={handleSeek}
          />
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => togglePlay()}
                className="rounded-full p-2 text-white hover:bg-white/10"
                aria-label={isPlaying ? 'Pause' : 'Play'}
              >
                {isPlaying ? <Pause className="h-5 w-5" /> : <Play className="h-5 w-5" />}
              </button>
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={volume}
                onChange={(e) => setVolume(Number(e.target.value))}
                className="scrub w-20"
                aria-label="Volume"
              />
            </div>

            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => setAudioOnly(!isAudioOnly)}
                className={`rounded-full p-2 hover:bg-white/10 ${isAudioOnly ? 'text-accent' : 'text-white'}`}
                aria-label="Toggle audio only"
                aria-pressed={isAudioOnly}
              >
                <Headphones className="h-5 w-5" />
              </button>
              <button
                type="button"
                onClick={() => setBackgroundAudio(!backgroundAudio)}
                className={`rounded-full p-2 hover:bg-white/10 ${backgroundAudio ? 'text-accent' : 'text-white'}`}
                aria-label="Background audio"
                aria-pressed={backgroundAudio}
              >
                <MonitorSpeaker className="h-5 w-5" />
              </button>
              <div className="relative">
                <button
                  type="button"
                  onClick={() => {
                    setShowSpeed(false)
                    setShowQuality((s) => !s)
                  }}
                  className="rounded-full p-2 text-white hover:bg-white/10"
                  aria-label="Quality"
                >
                  <Settings2 className="h-5 w-5" />
                </button>
                {showQuality ? (
                  <QualitySpeedMenu
                    label="Quality"
                    items={qualityItems}
                    selected={selectedQuality ?? activeFormat?.qualityLabel ?? ''}
                    onSelect={(v) => setSelectedQuality(v === 'audio' ? 'audio' : v)}
                    onClose={() => setShowQuality(false)}
                  />
                ) : null}
              </div>
              <div className="relative">
                <button
                  type="button"
                  onClick={() => {
                    setShowQuality(false)
                    setShowSpeed((s) => !s)
                  }}
                  className="rounded-full p-2 text-white hover:bg-white/10"
                  aria-label="Playback speed"
                >
                  <Gauge className="h-5 w-5" />
                </button>
                {showSpeed ? (
                  <QualitySpeedMenu
                    label="Speed"
                    items={speedItems}
                    selected={String(playbackRate)}
                    onSelect={(v) => setPlaybackRate(Number(v))}
                    onClose={() => setShowSpeed(false)}
                  />
                ) : null}
              </div>
              <button
                type="button"
                onClick={togglePiP}
                className="rounded-full p-2 text-white hover:bg-white/10"
                aria-label="Picture in picture"
              >
                <PictureInPicture2 className="h-5 w-5" />
              </button>
              <button
                type="button"
                onClick={toggleFullscreen}
                className="rounded-full p-2 text-white hover:bg-white/10"
                aria-label={isFullscreen ? 'Exit fullscreen' : 'Enter fullscreen'}
              >
                {isFullscreen ? <Minimize className="h-5 w-5" /> : <Maximize className="h-5 w-5" />}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
