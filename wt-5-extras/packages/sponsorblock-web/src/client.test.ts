import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';
import {
  SponsorBlockClient,
  Category,
  SponsorBlockError,
  sha256Prefix4,
} from './index.js';

const server = setupServer();

beforeAll(() => server.listen({ onUnhandledRequest: 'error' }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

const segmentFixture = {
  UUID: 'seg-1',
  start: 1,
  end: 5,
  category: 'sponsor',
  actionType: 'skip',
  videoDuration: 120,
  locked: 1,
  votes: 12,
};

const mockUrl = 'http://localhost:9999/api';

describe('SponsorBlockClient', () => {
  it('fetches segments by video id', async () => {
    server.use(
      http.get(`${mockUrl}/skipSegments`, ({ request }) => {
        const url = new URL(request.url);
        expect(url.searchParams.get('videoID')).toBe('abc123');
        expect(url.searchParams.get('categories')).toBe('["sponsor"]');
        return HttpResponse.json([segmentFixture]);
      }),
    );

    const client = new SponsorBlockClient({ base: mockUrl });
    const segments = await client.segments('abc123', [Category.Sponsor]);
    expect(segments).toHaveLength(1);
    expect(segments[0]?.uuid).toBe('seg-1');
    expect(segments[0]?.category).toBe('sponsor');
  });

  it('filters hash endpoint results by video id', async () => {
    const prefix = await sha256Prefix4('abc123');
    server.use(
      http.get(`${mockUrl}/skipSegments/:hash`, ({ params }) => {
        expect(params.hash).toBe(prefix);
        return HttpResponse.json([
          segmentFixture,
          { ...segmentFixture, UUID: 'seg-2', category: 'intro' },
        ]);
      }),
    );

    const client = new SponsorBlockClient({ base: mockUrl });
    const segments = await client.segmentsByHash('abc123', [
      Category.Sponsor,
    ]);
    expect(segments).toHaveLength(1);
    expect(segments[0]?.uuid).toBe('seg-1');
  });

  it('throws notFound for empty segments', async () => {
    server.use(
      http.get(`${mockUrl}/skipSegments`, () => HttpResponse.json([])),
    );

    const client = new SponsorBlockClient({ base: mockUrl });
    await expect(client.segments('none')).rejects.toSatisfy(
      (err) =>
        err instanceof SponsorBlockError && err.kind === 'notFound',
    );
  });

  it('submits a segment and returns the uuid', async () => {
    server.use(
      http.post(`${mockUrl}/skipSegments`, ({ request }) => {
        const url = new URL(request.url);
        expect(url.searchParams.get('videoID')).toBe('abc123');
        expect(url.searchParams.get('userID')).toBe('user-1');
        return HttpResponse.text('"new-uuid"');
      }),
    );

    const client = new SponsorBlockClient({ base: mockUrl });
    const uuid = await client.submit(
      'abc123',
      { start: 1, end: 5, category: Category.Sponsor },
      'user-1',
    );
    expect(uuid).toBe('new-uuid');
  });
});
