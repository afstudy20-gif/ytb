import { Link } from 'wouter'
import { Download, History, ThumbsUp, ListVideo, ChevronRight } from 'lucide-react'

const sections = [
  { href: '/library/downloads', label: 'Downloads', icon: Download, color: 'text-blue-400' },
  { href: '/library/history', label: 'History', icon: History, color: 'text-purple-400' },
  { href: '/library/liked', label: 'Liked videos', icon: ThumbsUp, color: 'text-green-400' },
  { href: '/library/playlists', label: 'Playlists', icon: ListVideo, color: 'text-yellow-400' },
]

export function Library() {
  return (
    <div className="flex flex-col gap-2 p-4">
      <h1 className="text-2xl font-bold text-text">Library</h1>
      {sections.map((section) => (
        <Link
          key={section.href}
          href={section.href}
          className="flex items-center gap-4 rounded-xl bg-surface p-4 transition-colors hover:bg-surface-hover focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
        >
          <section.icon className={`h-6 w-6 ${section.color}`} aria-hidden="true" />
          <span className="flex-1 text-base font-semibold text-text">{section.label}</span>
          <ChevronRight className="h-5 w-5 text-subtext" aria-hidden="true" />
        </Link>
      ))}
    </div>
  )
}
