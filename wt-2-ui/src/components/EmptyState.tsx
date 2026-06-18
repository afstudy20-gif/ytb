import type { LucideIcon } from 'lucide-react'

interface EmptyStateProps {
  icon: LucideIcon
  title: string
  subtitle: string
}

export function EmptyState({ icon: Icon, title, subtitle }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center px-6 py-16 text-center">
      <div className="mb-4 rounded-full bg-surface p-4">
        <Icon className="h-8 w-8 text-subtext" aria-hidden="true" />
      </div>
      <h3 className="text-lg font-semibold text-text">{title}</h3>
      <p className="mt-1 max-w-xs text-sm text-subtext">{subtitle}</p>
    </div>
  )
}
