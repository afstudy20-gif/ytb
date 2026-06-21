import { invoke } from '@tauri-apps/api/core'

export async function isAndroid(): Promise<boolean> {
  if (typeof navigator === 'undefined') return false
  return /Android/i.test(navigator.userAgent)
}

export interface VideoPlayerState {
  isPlaying: boolean
  positionMs: number
  durationMs: number
}

export async function openVideoPlayer(
  url: string,
  title: string,
  artist: string,
  artwork: string | undefined,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<void> {
  return invoke('open_video_player', { url, title, artist, artwork, x, y, width, height })
}

export async function closeVideoPlayer(): Promise<void> {
  return invoke('close_video_player')
}

export async function setVideoUrl(
  url: string,
  title: string,
  artist: string,
  artwork: string | undefined,
): Promise<void> {
  return invoke('set_video_url', { url, title, artist, artwork })
}

export async function setVideoBounds(x: number, y: number, width: number, height: number): Promise<void> {
  return invoke('set_video_bounds', { x, y, width, height })
}

export async function playVideo(): Promise<void> {
  return invoke('play_video')
}

export async function pauseVideo(): Promise<void> {
  return invoke('pause_video')
}

export async function seekVideo(positionMs: number): Promise<void> {
  return invoke('seek_video', { positionMs })
}

export async function getVideoState(): Promise<VideoPlayerState> {
  return invoke('get_video_state')
}
