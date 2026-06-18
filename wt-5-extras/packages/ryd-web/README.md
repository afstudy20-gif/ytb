# @wt-5/ryd-web

Return YouTube Dislike API client for the browser, with an in-memory LRU cache.

```ts
import { RydClient } from '@wt-5/ryd-web';

const client = new RydClient();
const votes = await client.votes('dQw4w9WgXcQ');
console.log(`${votes.likes} likes / ${votes.dislikes} dislikes`);
```
