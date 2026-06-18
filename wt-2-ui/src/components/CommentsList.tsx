import { ThumbsUp, ChevronDown } from 'lucide-react'
import { useState } from 'react'
import type { CommentThread } from '../lib/types.ts'
import { formatCount } from '../lib/format.ts'
import { Avatar } from './Avatar.tsx'
import { Skeleton } from './Skeleton.tsx'

interface CommentsListProps {
  threads: CommentThread[] | undefined
  isLoading: boolean
  totalCount: number
}

function CommentItem({ comment }: { comment: CommentThread['comment'] }) {
  return (
    <div className="flex gap-3 py-2">
      <Avatar src={comment.author.avatarUrl} alt={comment.author.name} size="sm" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold text-text">{comment.author.name}</span>
          <span className="text-xs text-subtext">{comment.publishedText}</span>
        </div>
        <p className="mt-0.5 text-sm text-text">{comment.text}</p>
        <div className="mt-1 flex items-center gap-3 text-subtext">
          <button type="button" className="flex items-center gap-1 text-xs hover:text-text" aria-label="Like comment">
            <ThumbsUp className="h-3.5 w-3.5" />
            {formatCount(comment.likeCount)}
          </button>
          {comment.replyCount > 0 ? (
            <button type="button" className="flex items-center gap-1 text-xs text-accent">
              <ChevronDown className="h-3.5 w-3.5" />
              {comment.replyCount} replies
            </button>
          ) : null}
        </div>
      </div>
    </div>
  )
}

export function CommentsList({ threads, isLoading, totalCount }: CommentsListProps) {
  const [expanded, setExpanded] = useState(false)

  if (isLoading) {
    return (
      <div className="px-4 py-2">
        <Skeleton className="mb-3 h-4 w-32" />
        {Array.from({ length: 3 }).map((_, i) => (
          <div key={i} className="flex gap-3 py-2">
            <Skeleton className="h-8 w-8 rounded-full" />
            <div className="flex-1 space-y-2">
              <Skeleton className="h-3 w-24" />
              <Skeleton className="h-3 w-full" />
            </div>
          </div>
        ))}
      </div>
    )
  }

  if (!threads?.length) return null

  const visible = expanded ? threads : threads.slice(0, 3)

  return (
    <section className="border-t border-border px-4 py-3">
      <div className="flex items-center justify-between">
        <h3 className="text-base font-bold text-text">Comments · {formatCount(totalCount)}</h3>
      </div>
      {visible.map((thread) => (
        <CommentItem key={thread.comment.id} comment={thread.comment} />
      ))}
      {threads.length > 3 ? (
        <button
          type="button"
          onClick={() => setExpanded((e) => !e)}
          className="mt-1 text-sm font-semibold text-accent"
        >
          {expanded ? 'Show less' : `Show all ${threads.length} comments`}
        </button>
      ) : null}
    </section>
  )
}
