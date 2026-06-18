export interface RegisterServiceWorkerOptions {
  /** Scope for the registered service worker. */
  scope?: string;
  /** URL of the service worker script. */
  url?: string;
}

/**
 * Register the application service worker if `navigator.serviceWorker`
 * is available.
 *
 * @param opts - registration options
 * @returns the active registration
 */
export async function registerServiceWorker(
  opts: RegisterServiceWorkerOptions = {},
): Promise<ServiceWorkerRegistration> {
  if (typeof navigator === 'undefined' || !navigator.serviceWorker) {
    return Promise.reject(
      new Error('service workers are not supported in this environment'),
    );
  }

  const { serviceWorker } = navigator;
  const registration = await serviceWorker.register(opts.url ?? '/sw.js', {
    scope: opts.scope ?? '/',
  });
  return registration;
}
