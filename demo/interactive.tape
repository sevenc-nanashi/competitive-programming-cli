# Run with `mise run demo demo/interactive.tape`.
Require cpg
Require asciinema
Require ruby

Set Shell bash
Set FontFamily "DejaVu Sans Mono"
Set FontSize 20
Set Width 1100
Set Height 500
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Hide
Type@1ms `export PS1='$ '; cd "$CPG_DEMO_WORKSPACE/interactive"; asciinema rec --quiet --overwrite --return --output-format asciicast-v3 --title 'cpg interactive: Guess the number' --command 'bash --noprofile --norc' "$CPG_DEMO_DESTINATION"`
Enter
Wait+Line /\$$/
Show

Type "# Guess the secret number in 1..10 using HIGHER / LOWER / EQUAL"
Enter
Type "cpg test -i -J judge.rb -v always -t 2000 solution.rb"
Enter
Wait+Line /\$$/
Sleep 4s

Type "# Judge on the left, solution on the right; arrows show direction"
Enter
Type "cpg test -i -J judge.rb -v always -t 2000 -p outputs solution.rb"
Enter
Wait+Line /\$$/
Sleep 5s

Type "# Add query numbers between the panes"
Enter
Type "cpg test -i -J judge.rb -v always -t 2000 -p outputs -n solution.rb"
Enter
Wait+Line /\$$/
Sleep 5s

Ctrl+D
