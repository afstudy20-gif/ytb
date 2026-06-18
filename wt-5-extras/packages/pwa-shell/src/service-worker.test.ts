import { describe, it, expect, vi } from 'vitest';
import { registerServiceWorker } from './index.js';

describe('registerServiceWorker', () => {
  it('registers a service worker', async () => {
    const mockRegistration = { scope: '/' } as ServiceWorkerRegistration;
    const register = vi.fn().mockResolvedValue(mockRegistration);
    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: { register },
      configurable: true,
      writable: true,
    });

    const result = await registerServiceWorker({ url: '/sw.js' });
    expect(register).toHaveBeenCalledWith('/sw.js', { scope: '/' });
    expect(result).toBe(mockRegistration);
  });

  it('rejects when service workers are unsupported', async () => {
    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: undefined,
      configurable: true,
      writable: true,
    });

    await expect(registerServiceWorker()).rejects.toThrow(
      'service workers are not supported',
    );
  });
});
