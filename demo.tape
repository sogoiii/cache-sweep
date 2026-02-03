# cache-sweep demo
# Run with: vhs demo.tape

Output demo.gif

Require cache-sweep

# Terminal settings
Set Shell "bash"
Set FontSize 14
Set LetterSpacing 1
Set Width 1400
Set Height 700
Set Theme "Dracula"
Set Padding 20
Set TypingSpeed 50ms
Set WindowBar Colorful

# Setup: cd to demo directory (hidden)
Hide
Type "cd ~/Documents/codebase/opensource"
Enter
Sleep 500ms
Show

# Show help first
Type "cache-sweep --help"
Sleep 500ms
Enter
Sleep 3s

# Clear and run the TUI
Type "clear"
Enter
Sleep 500ms

# Run cache-sweep on current directory
Type "cache-sweep"
Sleep 500ms
Enter

# Let results stream in
Sleep 4s

# Navigate down through results
Down 2
Sleep 500ms
Down 2
Sleep 500ms

# Open info panel with Right arrow (or 'l')
Right
Sleep 2s

# Close info panel with Left arrow
Left
Sleep 500ms

# Show search with '/'
Type "/"
Sleep 500ms
Type "effect"
Sleep 1s
Escape
Sleep 500ms

# Navigate more
Up 2
Sleep 500ms

# Show analytics panel with 'a'
Type "a"
Sleep 2s

# Close analytics (press 'a' again)
Type "a"
Sleep 500ms

# Show sort options with 's'
Type "s"
Sleep 1s
Down
Enter
Sleep 1s

# Quit
Type "q"
Sleep 1s
