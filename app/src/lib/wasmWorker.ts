// Runs the randomizer (wasm) off the page's main thread, so the UI stays
// smooth while a seed builds. Messages: {id, name, input, blobs} in,
// {id, json, blobs} or {id, error} out.

import { loadMmr } from "./wasm";

const mmr = loadMmr(fetch(new URL(`${import.meta.env.BASE_URL}mmr.wasm`, self.location.origin)).then((r) => {
  if (!r.ok) throw new Error("The randomizer isn't built for the browser here (mmr.wasm is missing: run npm run build:web).");
  return r;
}));

self.onmessage = async (e: MessageEvent) => {
  const { id, name, input, blobs } = e.data;
  try {
    const r = (await mmr).call(name, input, blobs);
    (self as unknown as Worker).postMessage({ id, json: r.json, blobs: r.blobs }, r.blobs.map((b) => b.buffer as ArrayBuffer));
  } catch (err) {
    (self as unknown as Worker).postMessage({ id, error: err instanceof Error ? err.message : String(err) });
  }
};
