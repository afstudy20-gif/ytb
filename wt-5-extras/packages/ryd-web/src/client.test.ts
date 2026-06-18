import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';
import { RydClient, RydError } from './index.js';

const server = setupServer();

beforeAll(() => server.listen({ onUnhandledRequest: 'error' }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

const votesFixture = {
  id: 'abc123',
  dateCreated: 1_700_000_000,
  likes: 100,
  dislikes: 5,
  rating: 4.8,
  viewCount: 10_000,
  deleted: false,
};

const mockUrl = 'http://localhost:9999';

describe('RydClient', () => {
  it('fetches votes from the server', async () => {
    server.use(
      http.get(`${mockUrl}/votes`, ({ request }) => {
        const url = new URL(request.url);
        expect(url.searchParams.get('videoId')).toBe('abc123');
        return HttpResponse.json(votesFixture);
      }),
    );

    const client = new RydClient({ base: mockUrl });
    const votes = await client.votes('abc123');
    expect(votes.id).toBe('abc123');
    expect(votes.likes).toBe(100);
    expect(votes.dislikes).toBe(5);
  });

  it('serves repeated requests from cache', async () => {
    let requests = 0;
    server.use(
      http.get(`${mockUrl}/votes`, () => {
        requests += 1;
        return HttpResponse.json(votesFixture);
      }),
    );

    const client = new RydClient({ base: mockUrl });
    await client.votes('abc123');
    await client.votes('abc123');
    expect(requests).toBe(1);
  });

  it('throws notFound for HTTP 404', async () => {
    server.use(
      http.get(`${mockUrl}/votes`, () => HttpResponse.json({}, { status: 404 })),
    );

    const client = new RydClient({ base: mockUrl });
    await expect(client.votes('missing')).rejects.toSatisfy(
      (err) => err instanceof RydError && err.kind === 'notFound',
    );
  });

  it('does not cache deleted records', async () => {
    let requests = 0;
    server.use(
      http.get(`${mockUrl}/votes`, () => {
        requests += 1;
        return HttpResponse.json({ ...votesFixture, deleted: true });
      }),
    );

    const client = new RydClient({ base: mockUrl });
    await client.votes('abc123');
    await client.votes('abc123');
    expect(requests).toBe(2);
  });
});
