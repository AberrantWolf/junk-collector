/* Experiment-only HTTP media adapter. The production player stays unchanged. */
self.addEventListener('install', event => event.waitUntil(self.skipWaiting()));
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));
const media = /^\/api\/v1\/items\/[0-9a-f-]{36}\/representations\/[0-9a-f-]{36}\/\d+$/;
const block = 262144;
async function read(client, value) {
  const channel = new MessageChannel();
  return await new Promise((resolve, reject) => {
    const timer = setTimeout(() => {channel.port1.close(); reject(Error('WebTransport read timeout'));}, 15000);
    channel.port1.onmessage = event => {
      clearTimeout(timer); channel.port1.close();
      if (event.data.error) reject(Error(event.data.error)); else resolve(event.data);
    };
    client.postMessage({type:'wt-read', value}, [channel.port2]);
  });
}
async function handle(event, path) {
  const client = await self.clients.get(event.clientId);
  if (!client) return new Response('No page transport owner', {status:503});
  client.postMessage({type:'adapter-event', value:`${event.request.method} media Range ${event.request.headers.get('range') || '(none)'}`});
  const head = await read(client, {path, method:'HEAD'});
  if (head.status !== 200) return new Response(null, {status:head.status});
  const headers = new Headers(head.headers);
  headers.set('cache-control', 'no-store');
  headers.set('x-experiment-transport', 'webtransport');
  const length = Number(headers.get('content-length'));
  if (!Number.isSafeInteger(length) || length <= 0) return new Response(null, {status:502});
  if (event.request.method === 'HEAD') return new Response(null, {status:200, headers});
  let start = 0, end = length - 1, status = 200;
  const range = event.request.headers.get('range');
  if (range) {
    const match = /^bytes=(\d*)-(\d*)$/.exec(range);
    if (!match || (!match[1] && !match[2])) return new Response(null, {status:416});
    start = match[1] ? Number(match[1]) : Math.max(0, length - Number(match[2]));
    end = match[1] && match[2] ? Math.min(Number(match[2]), length - 1) : length - 1;
    if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start > end || start >= length)
      return new Response(null, {status:416, headers:{'content-range':`bytes */${length}`}});
    status = 206;
    headers.set('content-range', `bytes ${start}-${end}/${length}`);
  }
  headers.set('content-length', String(end - start + 1));
  let offset = start, cancelled = false;
  const stream = new ReadableStream({
    async pull(controller) {
      try {
        if (cancelled) return;
        const last = Math.min(end, offset + block - 1);
        const result = await read(client, {path, method:'GET', range:`bytes=${offset}-${last}`});
        if (cancelled) return;
        if (result.status !== 206 || result.headers['content-range'] !== `bytes ${offset}-${last}/${length}`
            || result.body.byteLength !== last - offset + 1 || result.headers.etag !== head.headers.etag)
          throw Error(`Media read rejected (${result.status}) or representation changed`);
        controller.enqueue(new Uint8Array(result.body));
        offset = last + 1;
        if (offset > end) controller.close();
        // Deliberately throttle this synthetic fixture to make unbuffered seeks observable.
        else await new Promise(resolve => setTimeout(resolve, 400));
      } catch (error) {if (!cancelled) controller.error(error);}
    },
    cancel() {cancelled = true;client.postMessage({type:'adapter-event',value:`Cancelled media response at byte ${offset}`});}
  }, {highWaterMark:1});
  return new Response(stream, {status, headers});
}
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (url.origin === self.location.origin && media.test(url.pathname)
      && ['GET','HEAD'].includes(event.request.method))
    event.respondWith(handle(event, url.pathname).catch(error => new Response(String(error), {status:503})));
});
