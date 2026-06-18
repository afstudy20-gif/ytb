import { Home, Library, Search, Settings } from 'lucide-react'
import { Link, useLocation } from 'wouter'

const tabs = [
  { href: '/', label: 'Home', icon: Home },
  { href: '/search', label: 'Search', icon: Search },
  { href: '/library', label: 'Library', icon: Library },
  { href: '/settings', label: 'Settings', icon: Settings },
]

export function BottomNav() {
  const [location] = useLocation()

  return (
    <nav
      className="fixed bottom-0 left-0 right-0 z-40 border-t border-border bg-bg/95 backdrop-blur safe-bottom"
      aria-label="Primary"
    >
      <ul className="flex h-14 items-center justify-around">
        {tabs.map((tab) => {
          const active = location === tab.href || location.startsWith(`${tab.href}/`)
          return (
            <li key={tab.href} className="flex-1">
              <Link
                href={tab.href}
                className={`flex flex-col items-center justify-center gap-0.5 py-2 text-xs transition-colors ${
                  active ? 'text-accent' : 'text-subtext'
                }`}
                aria-current={active ? 'page' : undefined}
              >
                <tab.icon className="h-5 w-5" aria-hidden="true" />
                <span>{tab.label}</span>
              </Link>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}
