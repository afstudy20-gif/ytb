/**
 * Dislike / like payload returned by `GET /votes?videoId=...`.
 */
export interface Votes {
  /** YouTube video id this record describes. */
  id: string;
  /** Unix timestamp (seconds) the record was first created. */
  dateCreated: number;
  /** Like count at the time of the snapshot. */
  likes: number;
  /** Dislike count at the time of the snapshot. */
  dislikes: number;
  /** 1..=5 average rating derived from the above. */
  rating: number;
  /** View count at the time of the snapshot (0 when unknown). */
  viewCount: number;
  /** Whether the upstream record has been marked deleted. */
  deleted: boolean;
}
