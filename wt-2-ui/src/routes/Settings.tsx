import { useSettingsStore } from '../stores/settings.ts'
import type { DownloadQuality, AudioFormat, ThemeMode } from '../lib/types.ts'

const sponsorBlockOptions = [
  { value: 'sponsor', label: 'Sponsor' },
  { value: 'intro', label: 'Intro' },
  { value: 'outro', label: 'Outro' },
  { value: 'selfpromo', label: 'Self-promotion' },
  { value: 'interaction', label: 'Interaction reminder' },
  { value: 'music_offtopic', label: 'Music off-topic' },
]

const qualities: DownloadQuality[] = ['audio-only', '360p', '720p', '1080p']
const audioFormats: AudioFormat[] = ['m4a', 'opus']
const themes: ThemeMode[] = ['system', 'dark', 'light']

function Toggle({
  checked,
  onChange,
  label,
  description,
}: {
  checked: boolean
  onChange: () => void
  label: string
  description?: string
}) {
  return (
    <button
      type="button"
      onClick={onChange}
      className="flex w-full items-center justify-between rounded-xl bg-surface p-4 text-left"
    >
      <div>
        <div className="text-sm font-semibold text-text">{label}</div>
        {description ? <div className="text-xs text-subtext">{description}</div> : null}
      </div>
      <span
        className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
          checked ? 'bg-accent' : 'bg-border'
        }`}
        aria-hidden="true"
      >
        <span
          className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
            checked ? 'translate-x-6' : 'translate-x-1'
          }`}
        />
      </span>
    </button>
  )
}

export function Settings() {
  const {
    sponsorBlockCategories,
    returnYouTubeDislike,
    defaultDownloadQuality,
    defaultAudioFormat,
    wifiOnlyDownloads,
    theme,
    toggleSponsorBlockCategory,
    toggleReturnYouTubeDislike,
    setDefaultDownloadQuality,
    setDefaultAudioFormat,
    toggleWifiOnlyDownloads,
    setTheme,
  } = useSettingsStore()

  return (
    <div className="flex flex-col gap-3 p-4 pb-24">
      <h1 className="text-2xl font-bold text-text">Settings</h1>

      <section>
        <h2 className="mb-2 px-1 text-sm font-semibold uppercase tracking-wide text-subtext">
          Appearance
        </h2>
        <div className="rounded-xl bg-surface p-2">
          <label className="block px-2 pb-1 text-sm font-semibold text-text">Theme</label>
          <div className="grid grid-cols-3 gap-2 p-2">
            {themes.map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => setTheme(t)}
                className={`rounded-lg px-3 py-2 text-sm font-medium capitalize ${
                  theme === t ? 'bg-accent text-white' : 'bg-surface-hover text-text'
                }`}
              >
                {t}
              </button>
            ))}
          </div>
        </div>
      </section>

      <section>
        <h2 className="mb-2 px-1 text-sm font-semibold uppercase tracking-wide text-subtext">
          Playback
        </h2>
        <Toggle
          checked={returnYouTubeDislike}
          onChange={toggleReturnYouTubeDislike}
          label="Return YouTube Dislike"
          description="Show estimated dislike counts"
        />
      </section>

      <section>
        <h2 className="mb-2 px-1 text-sm font-semibold uppercase tracking-wide text-subtext">
          SponsorBlock
        </h2>
        <div className="rounded-xl bg-surface p-4">
          <p className="mb-2 text-xs text-subtext">Categories to skip or highlight</p>
          <div className="flex flex-wrap gap-2">
            {sponsorBlockOptions.map((option) => {
              const active = sponsorBlockCategories.includes(option.value)
              return (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => toggleSponsorBlockCategory(option.value)}
                  className={`rounded-full px-3 py-1.5 text-xs font-semibold ${
                    active ? 'bg-accent text-white' : 'bg-surface-hover text-text'
                  }`}
                >
                  {option.label}
                </button>
              )
            })}
          </div>
        </div>
      </section>

      <section>
        <h2 className="mb-2 px-1 text-sm font-semibold uppercase tracking-wide text-subtext">
          Downloads
        </h2>
        <div className="rounded-xl bg-surface p-4">
          <label className="block text-sm font-semibold text-text">Default quality</label>
          <div className="mt-2 grid grid-cols-4 gap-2">
            {qualities.map((q) => (
              <button
                key={q}
                type="button"
                onClick={() => setDefaultDownloadQuality(q)}
                className={`rounded-lg px-2 py-2 text-xs font-semibold ${
                  defaultDownloadQuality === q ? 'bg-accent text-white' : 'bg-surface-hover text-text'
                }`}
              >
                {q}
              </button>
            ))}
          </div>
          <label className="mt-4 block text-sm font-semibold text-text">Audio format</label>
          <div className="mt-2 grid grid-cols-2 gap-2">
            {audioFormats.map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => setDefaultAudioFormat(f)}
                className={`rounded-lg px-2 py-2 text-xs font-semibold ${
                  defaultAudioFormat === f ? 'bg-accent text-white' : 'bg-surface-hover text-text'
                }`}
              >
                {f}
              </button>
            ))}
          </div>
          <div className="mt-4">
            <Toggle
              checked={wifiOnlyDownloads}
              onChange={toggleWifiOnlyDownloads}
              label="Wi-Fi only downloads"
              description="Avoid using mobile data"
            />
          </div>
        </div>
      </section>
    </div>
  )
}
