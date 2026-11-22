## `hongguo-window-layout` in Rust

![hongguo-window-layout report card](card.svg)

| | |
| --- | --- |
| Unit | `src/native/_wrapper-window-layout.js` (TypeScript/JavaScript) |
| Recorded cases matched | 50 / 50 |
| Speedup, call included | 0.1x (geometric mean) |
| Fallback to the original | 0.0% (0 of 50 cases) |
| Switched on | off |

| Function | Cases | Original per call | Rust per call | Speedup |
| --- | ---: | ---: | ---: | ---: |
| `fitWindowBounds` | 31 | 816 ns | 4.5 µs | 0.2x |
| `getWindowScale` | 19 | 16 ns | 133 ns | 0.1x |

Measured with `rust-anything report` on x86_64, 12 cores, Windows: `diff` replays every recorded case, `bench` times them on both sides as callers call them. Reproduce: `rust-anything diff` and `rust-anything bench` in the crate folder.
