## `hongguo-app-update` in Rust

![hongguo-app-update report card](card.svg)

| | |
| --- | --- |
| Unit | `src/native/app-update.js` (TypeScript/JavaScript) |
| Recorded cases matched | 114 / 114 |
| Speedup, call included | 0.2x (geometric mean) |
| Fallback to the original | 0.0% (0 of 114 cases) |
| Switched on | off |

| Function | Cases | Original per call | Rust per call | Speedup |
| --- | ---: | ---: | ---: | ---: |
| `availableRelease` | 114 | 180 ns | 1.1 µs | 0.2x |

Measured with `rust-anything report` on x86_64, 12 cores, Windows: `diff` replays every recorded case, `bench` times them on both sides as callers call them. Reproduce: `rust-anything diff` and `rust-anything bench` in the crate folder.
