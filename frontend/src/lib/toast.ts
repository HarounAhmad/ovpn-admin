import { writable } from 'svelte/store';
export type Toast = { id: number; text: string };
export const toasts = writable<Toast[]>([]);
let seq = 1;
export function toast(text: string, ms = 3000) {
    const id = seq++; toasts.update(t => [...t, { id, text }]);
    setTimeout(() => toasts.update(t => t.filter(x => x.id !== id)), ms);
}
