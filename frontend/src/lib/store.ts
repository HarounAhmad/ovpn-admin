import { writable, derived } from 'svelte/store'
import { api } from './api'

type Session = { username: string, roles: string[] } | null;

export const session = writable<Session>(null);
export const loading = writable<boolean>(true);

export async function refreshSession(): Promise<void> {
    loading.set(true);
    try {
        const me = await api.get('/me');
        session.set(me);
    } catch {
        session.set(null);
    } finally {
        loading.set(false);
    }
}

export async function login(username: string, password: string): Promise<void> {
    await api.post('/auth/login', { username, password });
    await refreshSession();
}

export async function logout(): Promise<void> {
    await api.post('/auth/logout');
    session.set(null);
}

export const roles = derived(session, s => new Set(s?.roles ?? []));
export function hasRole(s: Session, r: string): boolean {
    return !!s?.roles?.includes(r);
}
