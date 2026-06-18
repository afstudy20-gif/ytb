/**
 * SponsorBlock segment categories.
 *
 * @see {@link https://wiki.sponsor.ajay.app/w/API_Docs#Categories}
 */
export type Category =
  | 'sponsor'
  | 'selfpromo'
  | 'interaction'
  | 'intro'
  | 'outro'
  | 'preview'
  | 'music_offtopic'
  | 'filler';

/**
 * Enum-like constants for SponsorBlock categories.
 */
export const Category: Record<
  'Sponsor' | 'SelfPromo' | 'Interaction' | 'Intro' | 'Outro' | 'Preview' | 'MusicOfftopic' | 'Filler',
  Category
> = {
  Sponsor: 'sponsor',
  SelfPromo: 'selfpromo',
  Interaction: 'interaction',
  Intro: 'intro',
  Outro: 'outro',
  Preview: 'preview',
  MusicOfftopic: 'music_offtopic',
  Filler: 'filler',
} as const;

/**
 * Vote direction for {@link SponsorBlockClient.vote}.
 */
export type Vote = 'up' | 'down' | 'skip';

/**
 * Wire values for vote directions.
 */
const voteValue: Record<Vote, number> = {
  up: 1,
  down: 0,
  skip: 20,
};

/**
 * @internal
 */
export function voteToWire(vote: Vote): number {
  return voteValue[vote];
}

/**
 * A single sponsored / filler segment returned by the SponsorBlock API.
 */
export interface Segment {
  /** Public UUID identifying the segment. */
  uuid: string;
  /** Start time in seconds. */
  start: number;
  /** End time in seconds. */
  end: number;
  /** Category, e.g. `"sponsor"`. */
  category: Category;
  /** Action type — usually `"skip"`. */
  actionType: string;
  /** Total video duration at submission time, if known. */
  videoDuration: number | undefined;
  /** Whether the segment is locked against further votes. */
  locked: number;
  /** Net score (upvotes minus downvotes). */
  votes: number;
}

/**
 * Input for {@link SponsorBlockClient.submit}.
 */
export interface NewSegment {
  /** Segment start in seconds. */
  start: number;
  /** Segment end in seconds. */
  end: number;
  /** Category for the new segment. */
  category: Category | string;
  /** Action type — defaults to `"skip"`. */
  actionType?: string;
}
