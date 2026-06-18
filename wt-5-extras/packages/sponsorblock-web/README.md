# @wt-5/sponsorblock-web

Fetch-based SponsorBlock API client for the browser.

```ts
import { SponsorBlockClient, Category } from '@wt-5/sponsorblock-web';

const client = new SponsorBlockClient();
const segments = await client.segmentsByHash('dQw4w9WgXcQ', [
  Category.Sponsor,
  Category.SelfPromo,
]);
```
