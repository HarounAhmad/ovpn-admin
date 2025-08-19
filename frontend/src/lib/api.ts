const API = '/api';

function getCookie(name: string): string | null {
    const m = document.cookie.match(new RegExp(`(?:^|; )${name}=([^;]*)`));
    return m ? decodeURIComponent(m[1]) : null;
}

async function ensureCsrf(): Promise<string> {
    const existing = getCookie('XSRF-TOKEN');
    if (existing) return existing;
    await fetch(`${API}/auth/csrf`, { credentials: 'include' });
    const token = getCookie('XSRF-TOKEN');
    if (!token) throw new Error('csrf');
    return token;
}

async function request(path: string, init: RequestInit = {}): Promise<Response> {
    return fetch(`${API}${path}`, { credentials: 'include', ...init });
}

export async function getJson<T>(path: string): Promise<T> {
    const r = await request(path);
    if (r.status === 401) throw new Error('unauthorized');
    if (!r.ok) throw new Error(`GET ${path} -> ${r.status}`);
    return r.json() as Promise<T>;
}

export async function postJson<T>(path: string, body: unknown): Promise<T | void> {
    const token = await ensureCsrf();
    const r = await request(path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': token },
        body: JSON.stringify(body),
    });
    if (r.status === 401) throw new Error('unauthorized');
    if (r.status === 204) return;
    if (!r.ok) throw new Error(`POST ${path} -> ${r.status}`);
    const ct = r.headers.get('content-type') || '';
    return ct.includes('application/json') ? r.json() : (undefined as unknown as T);
}

export async function putJson<T>(path: string, body: unknown): Promise<T | void> {
    const token = await ensureCsrf();
    const r = await request(path, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': token },
        body: JSON.stringify(body),
    });
    if (r.status === 401) throw new Error('unauthorized');
    if (!r.ok) throw new Error(`PUT ${path} -> ${r.status}`);
    return r.status === 204 ? undefined : r.json();
}

export async function postBlob(path: string, body: unknown): Promise<Blob> {
    const token = await ensureCsrf();
    const r = await request(path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': token },
        body: JSON.stringify(body),
    });
    if (r.status === 401) throw new Error('unauthorized');
    if (!r.ok) throw new Error(`POST ${path} -> ${r.status}`);
    return r.blob();
}

export async function login(username: string, password: string): Promise<void> {
    await ensureCsrf();
    const r = await request('/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': getCookie('XSRF-TOKEN')! },
        body: JSON.stringify({ username, password }),
    });
    if (r.status === 204) return;
    if (r.status === 403) throw new Error('forbidden');
    throw new Error(`login ${r.status}`);
}

export async function logout(): Promise<void> {
    const token = await ensureCsrf();
    await request('/auth/logout', { method: 'POST', headers: { 'X-CSRF-Token': token } });
}
