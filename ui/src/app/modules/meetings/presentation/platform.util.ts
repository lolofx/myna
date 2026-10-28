/**
 * True when the webview runs on macOS. Used to show the notes that only apply
 * there (the microphone permission that macOS drops when an unsigned app is
 * replaced). Defaults to false where `navigator` is missing.
 */
export function isMacOs(): boolean {
  if (typeof navigator === 'undefined') {
    return false;
  }
  return /mac/i.test(navigator.platform || navigator.userAgent);
}
