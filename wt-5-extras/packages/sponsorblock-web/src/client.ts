import { SponsorBlockError } from './error.js';
import { voteToWire } from './model.js';
import type { Category, NewSegment, Segment, Vote } from './model.js';
import { sha256Prefix4 } from './crypto.js';

const DEFAULT_BASE = 'https://sponsor.ajay.app/api';

function isSuccess(status: number): boolean {
  return status >= 200 && status < 300;
}

function mapStatus(status: number): SponsorBlockError {
  switch (status) {
    case 401:
    case 403:
      return new SponsorBlockError('forbidden', `HTTP ${status}`);
    case 404:
      return new SponsorBlockError('notFound', 'no segments found');
    case 429:
      return new SponsorBlockError('rateLimited', `HTTP ${status}`);
    default:
      return new SponsorBlockError('network', `HTTP ${status}`);
  }
}

function requireVideoId(videoId: string): void {
  if (videoId.trim().length === 0) {
    throw new SponsorBlockError('decode', 'videoId must not be empty');
  }
}

function requireUuid(uuid: string): void {
  if (uuid.trim().length === 0) {
    throw new SponsorBlockError('decode', 'segmentUuid must not be empty');
  }
}

function looksEmpty(body: string): boolean {
  const trimmed = body.trim();
  return trimmed.length === 0 || trimmed === '[]';
}

interface WireSegment {
  UUID?: string;
  uuid?: string;
  start?: number;
  end?: number;
  category?: string;
  actionType?: string;
  videoDuration?: number;
  locked?: number;
  votes?: number;
}

function toSegment(raw: WireSegment): Segment {
  const uuid = raw.UUID ?? raw.uuid;
  if (uuid === undefined || uuid.length === 0) {
    throw new SponsorBlockError('decode', 'segment missing UUID');
  }
  if (raw.start === undefined || raw.end === undefined) {
    throw new SponsorBlockError('decode', 'segment missing start/end');
  }
  return {
    uuid,
    start: raw.start,
    end: raw.end,
    category: (raw.category ?? 'sponsor') as Category,
    actionType: raw.actionType ?? 'skip',
    videoDuration: raw.videoDuration,
    locked: raw.locked ?? 0,
    votes: raw.votes ?? 0,
  };
}

function decodeSegments(body: string): Segment[] {
  if (looksEmpty(body)) {
    throw new SponsorBlockError('notFound', 'no segments found');
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    throw new SponsorBlockError('decode', message);
  }
  if (!Array.isArray(parsed)) {
    throw new SponsorBlockError('decode', 'expected array of segments');
  }
  try {
    return parsed.map((item) => toSegment(item as WireSegment));
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    throw new SponsorBlockError('decode', message);
  }
}

function joinCategories(categories: readonly Category[]): string {
  return JSON.stringify(categories);
}

export interface SponsorBlockClientOptions {
  /** Base URL for the SponsorBlock API. */
  base?: string;
}

/**
 * Async fetch-based client for the SponsorBlock API.
 */
export class SponsorBlockClient {
  private readonly base: string;

  constructor(options: SponsorBlockClientOptions = {}) {
    this.base = options.base ?? DEFAULT_BASE;
  }

  /**
   * Fetch segments for a video by exact `videoId`.
   *
   * @param videoId - YouTube video id
   * @param categories - categories to request; empty means all
   * @returns matching segments
   */
  async segments(
    videoId: string,
    categories: readonly Category[] = [],
  ): Promise<Segment[]> {
    requireVideoId(videoId);
    const params = new URLSearchParams({ videoID: videoId });
    if (categories.length > 0) {
      params.set('categories', joinCategories(categories));
    }
    const response = await fetch(`${this.base}/skipSegments?${params.toString()}`);
    return this.handleSegmentResponse(response);
  }

  /**
   * Privacy-preserving variant using the SHA-256 prefix endpoint.
   * The server only sees the first 4 hex chars of `SHA-256(videoId)`;
   * results are filtered client-side.
   *
   * @param videoId - YouTube video id
   * @param categories - categories to request; empty means all
   * @returns matching segments for this video
   */
  async segmentsByHash(
    videoId: string,
    categories: readonly Category[] = [],
  ): Promise<Segment[]> {
    requireVideoId(videoId);
    const prefix = await sha256Prefix4(videoId);
    const params = new URLSearchParams();
    if (categories.length > 0) {
      params.set('categories', joinCategories(categories));
    }
    const query = params.toString();
    const url = query
      ? `${this.base}/skipSegments/${prefix}?${query}`
      : `${this.base}/skipSegments/${prefix}`;
    const response = await fetch(url);
    const bucket = await this.handleSegmentResponse(response);
    const wanted =
      categories.length > 0
        ? new Set<string>(categories)
        : undefined;
    return wanted === undefined
      ? bucket
      : bucket.filter((segment) => wanted.has(segment.category));
  }

  /**
   * Cast a vote on an existing segment.
   *
   * @param segmentUuid - segment UUID
   * @param vote - vote direction
   * @param userId - private, stable user id
   */
  async vote(segmentUuid: string, vote: Vote, userId: string): Promise<void> {
    requireUuid(segmentUuid);
    const params = new URLSearchParams({
      UUID: segmentUuid,
      userID: userId,
      type: voteToWire(vote).toString(),
    });
    const response = await fetch(
      `${this.base}/voteOnSponsorTime?${params.toString()}`,
      { method: 'POST' },
    );
    if (!isSuccess(response.status)) {
      throw mapStatus(response.status);
    }
  }

  /**
   * Submit a new segment. Returns the new segment's UUID.
   *
   * @param videoId - YouTube video id
   * @param segment - segment to submit
   * @param userId - private, stable user id
   * @returns new segment UUID
   */
  async submit(
    videoId: string,
    segment: NewSegment,
    userId: string,
  ): Promise<string> {
    requireVideoId(videoId);
    const params = new URLSearchParams({
      videoID: videoId,
      userID: userId,
    });
    const body = {
      segment: [segment.start, segment.end],
      category: segment.category,
      actionType: segment.actionType ?? 'skip',
      userAgent: '@wt-5/sponsorblock-web',
    };
    const response = await fetch(`${this.base}/skipSegments?${params.toString()}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!isSuccess(response.status)) {
      throw mapStatus(response.status);
    }
    const text = await response.text();
    const uuid = text.trim().replace(/^"|"$/g, '');
    if (uuid.length === 0) {
      throw new SponsorBlockError('decode', 'server returned empty uuid');
    }
    return uuid;
  }

  private async handleSegmentResponse(response: Response): Promise<Segment[]> {
    if (!isSuccess(response.status)) {
      throw mapStatus(response.status);
    }
    const body = await response.text();
    return decodeSegments(body);
  }
}
