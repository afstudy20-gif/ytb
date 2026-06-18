import { useRef, useState, useCallback, useEffect } from 'react'
import { Loader2 } from 'lucide-react'

interface PullToRefreshProps {
  onRefresh: () => void | Promise<void>
  children: React.ReactNode
}

export function PullToRefresh({ onRefresh, children }: PullToRefreshProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [pulling, setPulling] = useState(false)
  const [offset, setOffset] = useState(0)
  const startY = useRef(0)
  const isPullingRef = useRef(false)

  const onTouchStart = useCallback((e: TouchEvent) => {
    if (window.scrollY > 0) return
    startY.current = e.touches[0].clientY
    isPullingRef.current = true
  }, [])

  const onTouchMove = useCallback((e: TouchEvent) => {
    if (!isPullingRef.current) return
    const y = e.touches[0].clientY
    const delta = Math.max(0, y - startY.current)
    if (delta > 0 && window.scrollY === 0) {
      setPulling(true)
      setOffset(Math.min(delta * 0.4, 80))
    }
  }, [])

  const onTouchEnd = useCallback(() => {
    isPullingRef.current = false
    if (offset > 60) {
      void Promise.resolve(onRefresh()).finally(() => {
        setOffset(0)
        setPulling(false)
      })
    } else {
      setOffset(0)
      setPulling(false)
    }
  }, [offset, onRefresh])

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    el.addEventListener('touchstart', onTouchStart, { passive: true })
    el.addEventListener('touchmove', onTouchMove, { passive: true })
    el.addEventListener('touchend', onTouchEnd)
    return () => {
      el.removeEventListener('touchstart', onTouchStart)
      el.removeEventListener('touchmove', onTouchMove)
      el.removeEventListener('touchend', onTouchEnd)
    }
  }, [onTouchStart, onTouchMove, onTouchEnd])

  return (
    <div ref={containerRef} className="relative">
      <div
        className="pointer-events-none absolute inset-x-0 top-0 z-10 flex items-center justify-center text-accent transition-transform"
        style={{ transform: `translateY(${offset - 40}px)` }}
      >
        <Loader2 className={`h-6 w-6 ${pulling ? 'animate-spin' : ''}`} aria-hidden="true" />
      </div>
      <div style={{ transform: `translateY(${offset}px)` }}>{children}</div>
    </div>
  )
}
