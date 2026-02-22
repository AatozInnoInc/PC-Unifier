# macOS platform behavior

This document describes macOS-specific behavior that affects rule authors and users. It complements the code and ADRs in `docs/decisions/`.

---

## Keycode asymmetry (capture vs injection)

On macOS, the same virtual key codes (CGKeyCode) are used for:

| Physical key   | Virtual key code | KeyCode produced by capture |
|----------------|------------------|-----------------------------|
| F13            | 0x69             | F13                         |
| PrintScreen    | 0x69             | F13                         |
| F14            | 0x6B             | F14                         |
| ScrollLock     | 0x6B             | F14                         |
| F15            | 0x71             | F15                         |
| Pause          | 0x71             | F15                         |

Capture always maps these codes to **F13**, **F14**, and **F15** respectively. The OS does not distinguish the physical key (F13 vs PrintScreen, etc.) at the event level.

**Implication for rule authors:** Rules that trigger on `PrintScreen`, `ScrollLock`, or `Pause` will **never fire on macOS**. On this platform, use **F13**, **F14**, and **F15** for those physical keys. Injection can target either the function key or the alternate key name where the mapping supports it; capture cannot.
