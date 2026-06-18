import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { AppSettings, DownloadQuality, AudioFormat, ThemeMode } from '../lib/types.ts'

const DEFAULT_SETTINGS: AppSettings = {
  sponsorBlockCategories: ['sponsor', 'intro', 'outro', 'selfpromo'],
  returnYouTubeDislike: true,
  defaultDownloadQuality: '720p',
  defaultAudioFormat: 'opus',
  wifiOnlyDownloads: true,
  theme: 'system',
}

interface SettingsState extends AppSettings {
  update: (patch: Partial<AppSettings>) => void
  setTheme: (theme: ThemeMode) => void
  setDefaultDownloadQuality: (quality: DownloadQuality) => void
  setDefaultAudioFormat: (format: AudioFormat) => void
  toggleSponsorBlockCategory: (category: string) => void
  toggleReturnYouTubeDislike: () => void
  toggleWifiOnlyDownloads: () => void
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...DEFAULT_SETTINGS,
      update: (patch) => set((state) => ({ ...state, ...patch })),
      setTheme: (theme) => set({ theme }),
      setDefaultDownloadQuality: (defaultDownloadQuality) => set({ defaultDownloadQuality }),
      setDefaultAudioFormat: (defaultAudioFormat) => set({ defaultAudioFormat }),
      toggleSponsorBlockCategory: (category) =>
        set((state) => {
          const has = state.sponsorBlockCategories.includes(category)
          const sponsorBlockCategories = has
            ? state.sponsorBlockCategories.filter((c) => c !== category)
            : [...state.sponsorBlockCategories, category]
          return { sponsorBlockCategories }
        }),
      toggleReturnYouTubeDislike: () =>
        set((state) => ({ returnYouTubeDislike: !state.returnYouTubeDislike })),
      toggleWifiOnlyDownloads: () =>
        set((state) => ({ wifiOnlyDownloads: !state.wifiOnlyDownloads })),
    }),
    {
      name: 'yt-vanced-settings',
    },
  ),
)
