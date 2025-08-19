// minimal fetch w/ credentials + CSRF header
function xsrf(): string | null {
    const m = document.cookie.match(/(?:^|;\s*)XSRF-TOKEN=([^;]+)/);
    return m ? decodeURIComponent(m[1]) : null;
}

async function ensureCsrf(): Promise<void> {
    if (xsrf()) return;
    await fetch('/api/auth/csrf', { credentials: 'include' });
}

async function req(method: string, path: string, body?: any, asBlob = false) {
    await ensureCsrf();
    const headers: Record<string, string> = { 'Accept': 'application/json' };
    const token = xsrf();
    if (token) headers['X-CSRF-Token'] = token;
    if (body && !asBlob) headers['Content-Type'] = 'application/json';

    const r = await fetch(`/api${path}`, {
        method,
        headers,
        body: body ? (asBlob ? body : JSON.stringify(body)) : undefined,
        credentials: 'include'
    });

    if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);

    if (asBlob) return await r.blob();
    const ct = r.headers.get('content-type') || '';
    return ct.includes('application/json') ? r.json() : r.text();
}

export const api = {
    get: <T=any>(p: string) => req('GET', p) as Promise<T>,
    post: <T=any>(p: string, b?: any) => req('POST', p, b) as Promise<T>,
    put:  <T=any>(p: string, b?: any) => req('PUT',  p, b) as Promise<T>,
    del:  <T=any>(p: string) => req('DELETE', p) as Promise<T>,
    postBlob: (p: string, b?: any) => req('POST', p, b, true) as Promise<Blob>,
};
