# fastC capability-typed I/O demo
# Renders to assets/demo.gif via `vhs assets/demo.tape`.
#
# Storyboard (≤45 seconds):
#  1. Show a fastC program that calls time::now() without a cap.
#  2. Try to compile → type error names the missing argument.
#  3. Show the fixed version that takes a cap argument.
#  4. Compile → success.
#
# This is the cap-typed I/O wedge in 45 seconds: the compiler
# refused to call a side-effecting function without an explicit
# capability token, end of story.

Output assets/demo.gif

Set FontSize 16
Set Width 1000
Set Height 600
Set Theme "Catppuccin Mocha"
Set PlaybackSpeed 1.0
Set TypingSpeed 50ms

# --- Setup ---
Hide
Type "cd /tmp && rm -rf demo && mkdir demo && cd demo"
Enter
Type "alias fastc='/Volumes/Github/fastC/target/release/fastc'"
Enter
Type "cp /Volumes/Github/fastC/assets/demo_src/bad.fc main.fc"
Enter
Type "clear"
Enter
Show

# --- Act 1: show the buggy source ---
Type "cat main.fc"
Enter
Sleep 4s

# --- Act 2: try to compile → cap-flow error ---
Type "fastc compile main.fc -o main.c"
Enter
Sleep 4s

# --- Act 3: fix and re-compile ---
Type "cp /Volumes/Github/fastC/assets/demo_src/good.fc main.fc"
Enter
Sleep 500ms
Type "cat main.fc"
Enter
Sleep 4s

Type "fastc compile main.fc -o main.c && echo OK"
Enter
Sleep 3s
