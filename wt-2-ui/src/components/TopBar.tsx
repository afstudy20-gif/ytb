import { Menu, Cast } from 'lucide-react'
import { Avatar } from './Avatar.tsx'

export function TopBar() {
  return (
    <header className="sticky top-0 z-30 flex h-14 items-center justify-between border-b border-border bg-bg/95 px-4 backdrop-blur safe-top">
      <div className="flex items-center gap-2">
        <Menu className="h-6 w-6 text-text" aria-hidden="true" />
        <span className="text-xl font-bold tracking-tight text-text">
          Vanced<span className="text-accent">Tube</span>
        </span>
      </div>
      <div className="flex items-center gap-4">
        <button
          type="button"
          className="rounded-full p-2 text-text transition-colors hover:bg-surface-hover"
          aria-label="Cast"
        >
          <Cast className="h-5 w-5" aria-hidden="true" />
        </button>
        <Avatar
          src="https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=120&q=80"
          alt="Profile"
          size="sm"
        />
      </div>
    </header>
  )
}
