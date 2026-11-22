## `hongguo-media-sources` in Rust

![hongguo-media-sources report card](card.svg)

| | |
| --- | --- |
| Unit | `src/native/media-sources.js` (TypeScript/JavaScript) |
| Recorded cases matched | 244 / 244 |
| Speedup, call included | 0.4x (geometric mean) |
| Fallback to the original | 0.0% (0 of 244 cases) |
| Switched on | off |

| Function | Cases | Original per call | Rust per call | Speedup |
| --- | ---: | ---: | ---: | ---: |
| `getMediaSources` | 24 | 974 ns | 3.4 µs | 0.3x |
| `setBuiltinSourceEnabled` | 220 | 1.1 µs | 2.4 µs | 0.4x |

Measured with `rust-anything report` on x86_64, 12 cores, Windows: `diff` replays every recorded case, `bench` times them on both sides as callers call them. Reproduce: `rust-anything diff` and `rust-anything bench` in the crate folder.
