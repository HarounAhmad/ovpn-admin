import { writable } from 'svelte/store';
import { getJson } from './api';

export type Sess = { username: string; roles: string[] } | null;

export const session = writable<Sess>(null);
export const loading = writable<boolean>(true);

export function hasRole(s: Sess, role: string) {
    return !!(s && s.roles && s.roles.includes(role));
}

export async function refreshSession() {
    loading.set(true);
    try {
        const me = await getJson<{ username:string; roles:string[] }>('/me');
        session.set(me);
    } catch (_) {
        session.set(null);
    } finally {
        loading.set(false);
    }
}
