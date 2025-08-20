import { writable } from 'svelte/store';

function norm(p: string) {
    if (!p) return '/';
    if (!p.startsWith('/')) p = '/' + p;
    p = p.replace(/\/{2,}/g, '/');
    if (p.length > 1 && p.endsWith('/')) p = p.slice(0, -1);
    return p;
}

function readHash(): string {
    return norm(location.hash.slice(1));
}

export const route = writable<string>(readHash());

let current = readHash();
route.set(current);

window.addEventListener('hashchange', () => {
    const next = readHash();
    if (next !== current) {
        current = next;
        route.set(next);
    }
});

export function goto(path: string) {
    const next = norm(path);
    const now = readHash();
    if (next !== now) location.hash = next;
}
