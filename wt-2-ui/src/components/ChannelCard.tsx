import { Link } from 'wouter'
import { BadgeCheck } from 'lucide-react'
import type { ChannelSummary } from '../lib/types.ts'
import { formatCount } from '../lib/format.ts'
import { Avatar } from './Avatar.tsx'

interface ChannelCardProps {
  channel: ChannelSummary
}

export function ChannelCard({ channel }: ChannelCardProps) {
  return (
    <Link
      href={`/channel/${encodeURIComponent(channel.id)}`}
      className="flex items-center gap-3 rounded-xl p-3 transition-colors hover:bg-surface-hover focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
    >
      <Avatar src={channel.avatarUrl} alt={channel.name} size="lg" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1">
          <h3 className="truncate text-sm font-semibold text-text">{channel.name}</h3>
          {channel.verified ? (
            <BadgeCheck className="h-3 w-3 shrink-0 text-subtext" aria-label="Verified" />
          ) : null}
        </div>
        <p className="text-xs text-subtext">
          {formatCount(channel.subscriberCount)} subscribers · {channel.videoCount} videos
        </p>
        <p className="line-clamp-1 text-xs text-subtext">{channel.descriptionShort}</p>
      </div>
    </Link>
  )
}
