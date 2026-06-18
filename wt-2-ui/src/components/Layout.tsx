import { BottomNav } from './BottomNav.tsx'
import { TopBar } from './TopBar.tsx'

interface LayoutProps {
  children: React.ReactNode
  hideTopBar?: boolean
}

export function Layout({ children, hideTopBar = false }: LayoutProps) {
  return (
    <div className="flex min-h-screen flex-col bg-bg">
      {!hideTopBar ? <TopBar /> : null}
      <main className="flex-1 pb-24">{children}</main>
      <BottomNav />
    </div>
  )
}
