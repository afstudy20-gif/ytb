/**
 * Compute the first 4 uppercase hex characters of the SHA-256 digest of
 * `input`. This prefix is used by the privacy-preserving
 * `/skipSegments/{hashPrefix}` endpoint.
 */
export async function sha256Prefix4(input: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(input);
  const digest = await crypto.subtle.digest('SHA-256', data);
  const bytes = new Uint8Array(digest);
  const hex = Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
  return hex.slice(0, 4).toUpperCase();
}
