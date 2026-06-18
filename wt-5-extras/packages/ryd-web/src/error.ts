/**
 * Discriminated kinds for {@link RydError}.
 */
export type RydErrorKind =
  | 'network'
  | 'decode'
  | 'notFound'
  | 'rateLimited'
  | 'invalidInput';

/**
 * Typed error raised by {@link RydClient} operations.
 */
export class RydError extends Error {
  declare readonly name: 'RydError';

  /**
   * @param kind - machine-readable error category
   * @param message - optional human-readable detail
   */
  constructor(
    public readonly kind: RydErrorKind,
    message?: string,
  ) {
    super(message ?? kind);
    this.name = 'RydError';
  }
}
