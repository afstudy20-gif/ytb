interface SkeletonProps {
  className?: string
}

export function Skeleton({ className = '' }: SkeletonProps) {
  return (
    <div
      className={`animate-shimmer rounded bg-gradient-to-r from-surface via-surface-hover to-surface bg-[length:200%_100%] ${className}`}
    />
  )
}

export function VideoCardSkeleton() {
  return (
    <div className="flex flex-col gap-2 p-3">
      <Skeleton className="aspect-video w-full rounded-lg" />
      <div className="flex gap-3 pt-1">
        <Skeleton className="h-9 w-9 rounded-full" />
        <div className="flex flex-1 flex-col gap-2">
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-3 w-2/3" />
        </div>
      </div>
    </div>
  )
}

export function SearchResultSkeleton() {
  return (
    <div className="flex gap-3 p-3">
      <Skeleton className="h-24 w-40 shrink-0 rounded-lg" />
      <div className="flex flex-1 flex-col gap-2">
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-3 w-3/4" />
        <Skeleton className="h-3 w-1/2" />
      </div>
    </div>
  )
}
