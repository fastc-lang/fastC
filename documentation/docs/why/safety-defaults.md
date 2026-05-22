# Safety defaults

## Is "no recursion / no dynamic allocation" really the default?

No. Those are NASA/JPL Power of 10 rules for `--safety-level=critical`. The default is `--safety-level=standard`, which permits recursion and `fc_alloc` — appropriate for almost all fastC code, including agent runtimes that are inherently allocator-heavy. Critical mode is opt-in for the embedded / safety-critical niche where Rust is not competing hard.

## What's on by default in `standard` mode?

| Power of 10 rule | Standard | Critical |
|---|:-:|:-:|
| 1: No recursion | — | ✓ |
| 2: Bounded loops | ✓ | ✓ |
| 3: No dynamic allocation | — | ✓ |
| 4: Function size limit (60 lines) | ✓ | ✓ |
| 5: Assertion density | planned | planned |
| 6: Minimal scope | by design | by design |
| 7: Check return values | by design | by design |
| 8: No preprocessor | by design | by design |
| 9: Single-level pointers | ✓ | ✓ |
| 10: Zero warnings (`--strict`) | opt-in | opt-in |

Three columns of "by design" rules are baked into the language and don't need to be opted-into — fastC has no preprocessor, no implicit conversions (no truncation surprises), and no swallowed return values (every call result either binds to a let, gets explicitly `discard`-ed, or is the function's return value).

## How do I check what's enforced for my safety level?

```bash
fastc p10-rules --safety-level=critical
```

Lists every rule with its enabled / disabled / planned state for the requested level.

## Why is Power of 10 even in the picture?

fastC's design is rooted in NASA/JPL's "Power of 10" rules for safety-critical code, developed by Gerard Holzmann for the Mars Science Laboratory mission. We treat the rules as a maturity ceiling: standard mode picks the rules that pay for themselves everywhere; critical mode enables the rest for code where the answer to "is this allowed to allocate?" is a hard no.

Critical mode is not the default because it would make most real-world fastC code ergonomically painful for no proportionate safety gain. Agent runtimes need `vec::push` (allocates). Compilers need recursion. Most working programs need to choose `--safety-level=critical` only when their domain requires it.
