// minimal fetch w/ credentials + CSRF header
function xsrf(): string | null {
    const m = document.cookie.match(/(?:^|;\s*)XSRF-TOKEN=([^;]+)/);
    return m ? decodeURIComponent(m[1]) : null;
}

async function ensureCsrf(): Promise<void> {
    if (xsrf()) return;
    await fetch('/api/auth/csrf', { credentials: 'include' });
}


function isJsonBody(b: unknown): boolean {
    return b !== undefined
        && !(b instanceof FormData)
        && !(b instanceof Blob)
        && !(b instanceof ArrayBuffer)
        && !(b instanceof URLSearchParams)
        // typed arrays
        && !(ArrayBuffer.isView(b as any));
}

// Core requester
async function req(method: string, path: string, body?: any, expectBlob = false) {
    await ensureCsrf();

    const headers: HeadersInit = { Accept: 'application/json' };
    const token = xsrf();
    if (token) (headers as Record<string, string>)['X-CSRF-Token'] = token;

    let payload: BodyInit | null = null;
    if (body !== undefined) {
        if (isJsonBody(body)) {
            (headers as Record<string, string>)['Content-Type'] = 'application/json';
            payload = JSON.stringify(body);
        } else {
            payload = body as BodyInit;
        }
    }

    const init: {
        headers: Record<string, string>;
        method: string;
        credentials: string;
        body: null | string | ReadableStream<any> | Blob | ArrayBufferView<ArrayBufferLike> | ArrayBuffer | FormData | URLSearchParams
    } = {
        method,
        headers,
        credentials: 'include',
        body: payload,
    };

    const r = await fetch(`/api${path}`, init);
    if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);

    if (expectBlob) return await r.blob();

    const ct = r.headers.get('content-type') ?? '';
    return ct.includes('application/json') ? r.json() : r.text();
}


export const api = {
    get: <T=any>(p: string) => req('GET', p) as Promise<T>,
    post: <T=any>(p: string, b?: any) => req('POST', p, b) as Promise<T>,
    put:  <T=any>(p: string, b?: any) => req('PUT',  p, b) as Promise<T>,
    del:  <T=any>(p: string) => req('DELETE', p) as Promise<T>,
    postBlob: (p: string, b?: any)  => req('POST', p, b, true) as Promise<Blob>,

};
