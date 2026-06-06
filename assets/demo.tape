# fastC v1.0 demo — capability-typed I/O wedge + v1.3 agent tooling
# Renders to assets/demo.gif via `vhs assets/demo.tape`.
#
# Three acts (≤70 seconds total):
#  Act 1 (~22s): cap-typed I/O wedge — bad → compile error → fixed.
#  Act 2 (~26s): v1.3 agent tooling — explain JSON / context / fix.
#  Act 3 (~12s): fastc-core ecosystem — fastc.toml with 5 of 11 deps.

Output assets/demo.gif

Set FontSize 16
Set Width 1000
Set Height 600
Set Theme "Catppuccin Mocha"
Set PlaybackSpeed 1.0
Set TypingSpeed 60ms

# ---- Setup (hidden) ----
Hide
Type "cd /tmp && rm -rf demo && mkdir demo && cd demo"
Enter
Type "alias fastc='/Volumes/Github/fastC/target/release/fastc'"
Enter
Type "cp /Volumes/Github/fastC/assets/demo_src/bad.fc main.fc"
Enter
Type "cp /Volumes/Github/fastC/assets/demo_src/good.fc good.fc"
Enter
Type "cp /Volumes/Github/fastC/assets/demo_src/explain_demo.fc demo.fc"
Enter
Type "cp /Volumes/Github/fastC/assets/demo_src/broken.fc broken.fc"
Enter
Type "cp /Volumes/Github/fastC/assets/demo_src/manifest_demo/fastc.toml fastc.toml"
Enter
Type "clear"
Enter
Show

# ==== ACT 1: capability-typed I/O wedge ====

Type "cat main.fc"
Enter
Sleep 3500ms

Type "fastc compile main.fc -o main.c"
Enter
Sleep 4s

Type "cp good.fc main.fc && cat main.fc"
Enter
Sleep 4s

Type "fastc compile main.fc -o main.c && echo OK"
Enter
Sleep 2500ms

Type "clear"
Enter

# ==== ACT 2: v1.3 agent tooling ====

Type "fastc explain demo.fc | head -28"
Enter
Sleep 5s

Type "fastc context demo.fc | head -16"
Enter
Sleep 5s

Type "fastc fix broken.fc --dry-run"
Enter
Sleep 5s

Type "clear"
Enter

# ==== ACT 3: fastc-core ecosystem ====

Type "cat fastc.toml"
Enter
Sleep 5s

Type "echo 'fastc-core: 11 packages — cli log json toml http time base64 uuid crypto-primitives regex sqlite'"
Enter
Sleep 3s

Type "echo 'fastC v1.0 — github.com/Skelf-Research/fastC'"
Enter
Sleep 2s
