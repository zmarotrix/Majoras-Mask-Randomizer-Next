// The randomizer compiled to WebAssembly (crates/mmr-web), for the browser
// build: JSON and byte blobs in, JSON and byte blobs out.

export interface WasmResult {
  json: any;
  blobs: Uint8Array[];
}

export interface Mmr {
  call(name: string, input?: unknown, blobs?: Uint8Array[]): WasmResult;
}

interface Exports {
  memory: WebAssembly.Memory;
  mmr_alloc(len: number): number;
  mmr_push_blob(ptr: number, len: number): void;
  mmr_call(name: number, nameLen: number, input: number, inputLen: number): number;
  mmr_out_json_ptr(): number;
  mmr_out_json_len(): number;
  mmr_out_blob_count(): number;
  mmr_out_blob_ptr(i: number): number;
  mmr_out_blob_len(i: number): number;
  mmr_out_clear(): void;
}

/** Load the module from its bytes (or a fetch Response). */
export async function loadMmr(source: BufferSource | Response | Promise<Response>): Promise<Mmr> {
  const resolved = await source;
  const { instance } =
    resolved instanceof Response
      ? await WebAssembly.instantiate(await resolved.arrayBuffer(), {})
      : await WebAssembly.instantiate(resolved, {});
  const e = instance.exports as unknown as Exports;
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  // The memory can grow during a call, so take a fresh view each time.
  const mem = () => new Uint8Array(e.memory.buffer);
  const put = (bytes: Uint8Array) => {
    const p = e.mmr_alloc(bytes.length) >>> 0;
    mem().set(bytes, p);
    return p;
  };
  const copy = (ptr: number, len: number) => mem().slice(ptr >>> 0, (ptr >>> 0) + len);
  return {
    call(name, input = null, blobs = []) {
      for (const b of blobs) e.mmr_push_blob(put(b), b.length);
      const n = enc.encode(name);
      const j = enc.encode(input == null ? "" : JSON.stringify(input));
      const status = e.mmr_call(put(n), n.length, put(j), j.length);
      const json = JSON.parse(dec.decode(copy(e.mmr_out_json_ptr(), e.mmr_out_json_len())));
      const out: Uint8Array[] = [];
      for (let i = 0; i < e.mmr_out_blob_count(); i++) out.push(copy(e.mmr_out_blob_ptr(i), e.mmr_out_blob_len(i)));
      e.mmr_out_clear();
      if (status !== 0) throw new Error(json?.error ?? "The randomizer failed.");
      return { json, blobs: out };
    },
  };
}
