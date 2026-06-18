import { Route, Switch } from 'wouter'
import { Layout } from './components/Layout.tsx'
import { ThemeProvider } from './components/ThemeProvider.tsx'
import { MiniPlayer } from './components/MiniPlayer.tsx'
import { useInitStores } from './hooks/useInitStores.ts'
import { Home } from './routes/Home.tsx'
import { Search } from './routes/Search.tsx'
import { Watch } from './routes/Watch.tsx'
import { Library } from './routes/Library.tsx'
import { Downloads } from './routes/Downloads.tsx'
import { History } from './routes/History.tsx'
import { Liked } from './routes/Liked.tsx'
import { Playlists } from './routes/Playlists.tsx'
import { Settings } from './routes/Settings.tsx'

function App() {
  useInitStores()

  return (
    <ThemeProvider>
      <Layout>
        <Switch>
          <Route path="/" component={Home} />
          <Route path="/search" component={Search} />
          <Route path="/watch/:videoId" component={Watch} />
          <Route path="/library" component={Library} />
          <Route path="/library/downloads" component={Downloads} />
          <Route path="/library/history" component={History} />
          <Route path="/library/liked" component={Liked} />
          <Route path="/library/playlists" component={Playlists} />
          <Route path="/settings" component={Settings} />
        </Switch>
      </Layout>
      <MiniPlayer />
    </ThemeProvider>
  )
}

export default App
