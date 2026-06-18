/**
 * Discriminated kinds for {@link SponsorBlockError}.
 */
export type SponsorBlockErrorKind =
  | 'network'
  | 'decode'
  | 'notFound'
  | 'rateLimited'
  | 'forbidden';

/**
 * Typed error raised by {@link SponsorBlockClient} operations.
 */
export class SponsorBlockError extends Error {
  declare readonly name: 'SponsorBlockError';

  /**
   * @param kind - machine-readable error category
   * @param message - optional human-readable detail
   */
  constructor(
    public readonly kind: SponsorBlockErrorKind,
    message?: string,
  ) {
    super(message ?? kind);
    this.name = 'SponsorBlockError';
  }
}
