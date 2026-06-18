import { useRef, useMemo } from 'react'
import type { SponsorSegment, Chapter } from '../lib/types.ts'
import { formatDuration } from '../lib/format.ts'

interface ScrubBarProps {
  currentTime: number
  duration: number
  segments: SponsorSegment[]
  chapters: Chapter[]
  onSeek: (time: number) => void
}

const categoryColors: Record<string, string> = {
  sponsor: 'bg-green-500',
  intro: 'bg-blue-500',
  outro: 'bg-blue-500',
  selfpromo: 'bg-yellow-500',
  interaction: 'bg-purple-500',
  music_offtopic: 'bg-orange-500',
}

export function ScrubBar({ currentTime, duration, segments, chapters, onSeek }: ScrubBarProps) {
  const trackRef = useRef<HTMLDivElement>(null)
  const progress = duration > 0 ? (currentTime / duration) * 100 : 0

  const markers = useMemo(() => {
    return segments.map((seg) => ({
      left: (seg.segment[0] / duration) * 100,
      width: ((seg.segment[1] - seg.segment[0]) / duration) * 100,
      color: categoryColors[seg.category] ?? 'bg-gray-400',
    }))
  }, [segments, duration])

  const handleClick = (e: React.MouseEvent | React.TouchEvent) => {
    if (!trackRef.current || duration <= 0) return
    const rect = trackRef.current.getBoundingClientRect()
    const clientX = 'touches' in e ? e.touches[0].clientX : e.clientX
    const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width))
    onSeek(ratio * duration)
  }

  return (
    <div className="group relative w-full py-2">
      <div
        ref={trackRef}
        className="relative h-1.5 w-full cursor-pointer rounded bg-white/20"
        onClick={handleClick}
        onTouchStart={handleClick}
        role="slider"
        aria-valuemin={0}
        aria-valuemax={duration}
        aria-valuenow={currentTime}
        aria-label="Seek"
      >
        <div
          className="absolute left-0 top-0 h-full rounded bg-accent"
          style={{ width: `${progress}%` }}
        />
        {markers.map((m, i) => (
          <div
            key={i}
            className={`absolute top-0 h-full rounded ${m.color}`}
            style={{ left: `${m.left}%`, width: `${Math.max(m.width, 0.4)}%` }}
            title="SponsorBlock segment"
          />
        ))}
        {chapters.map((chapter, i) => (
          <div
            key={`c-${i}`}
            className="absolute top-0 h-full w-0.5 bg-white/50"
            style={{ left: `${(chapter.startSeconds / duration) * 100}%` }}
          />
        ))}
        <div
          className="absolute top-1/2 h-4 w-4 -translate-y-1/2 rounded-full bg-accent opacity-0 transition-opacity group-hover:opacity-100"
          style={{ left: `${progress}%`, transform: `translate(-50%, -50%)` }}
        />
      </div>
      <div className="flex justify-between text-xs text-white/80">
        <span>{formatDuration(currentTime)}</span>
        <span>{formatDuration(duration)}</span>
      </div>
    </div>
  )
}
