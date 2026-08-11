import { demoApi } from './demo-api';
import { nativeApi } from './tauri-native';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const tauriApi = isTauri ? nativeApi : demoApi;
