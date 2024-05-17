# Prettypretty

Prettypretty is a Python package for building awesome terminal user interfaces.


## Prettypretty Illustrated

The first screenshot illustrates prettypretty's support for maximizing text
contrast by comparing against backgrounds in all 216 colors from the 6x6x6 RGB
cube of 8-bit terminal colors.

![Background color
grid](https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/rgb6-background.png)


The second screenshot illustrates the reverse challenge, with prettypretty
picking the background color to maximize contrast for text in all 216 colors
from the 6x6x6 RGB cube. If you compare with the previous screenshot, you may
notice that prettypretty's contrast metric,
[APCA](https://github.com/Myndex/apca-w3), is *not* symmetric. That just why it
is more accurate than, say, the WCAG 2.0 formula.

![Text color
grid](https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/rgb6-text.png)


The third screenshot illustrates prettypretty's support for finding the
perceptually closest color out of several colors. That's just how prettypretty
performs high-quality downsampling, in this case turning the 216 colors from the
6x6x6 RGB cube into 16 extended ANSI colors.

![Downsampled colors, macOS
Terminal](https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/rgb6-ansi-macos.png)


Since almost all terminals have robust support for theming just those 16
extended ANSI colors, prettypretty doesn't just use some hardcoded set of colors
but has built-in support for color themes. You can of course configure and
reconfigure the current colors as you please. But prettypretty can do one
better: It can automatically query a terminal for the current theme colors.
The fourth screenshot illustrates the impact. When running in iTerm2 instead of
macOS Terminal, prettypretty makes good use of the brighter colors in one of
iTerm's builtin themes and generates a substantially different grid!

![Downsampled colors,
iTerm2](https://raw.githubusercontent.com/apparebit/prettypretty/main/docs/figures/rgb6-ansi-iterm2.png)


To recap, prettypretty has robust support for:

  * Maximizing the label contrast for a given background color;
  * Maximizing the background contrast for a given text color;
  * Finding the closest color out of several;
  * Using that search to perform high-quality downsampling;
  * Theming the sixteen extended ANSI colors;
  * Automatically determining the current terminal theme.

More is yet to come...





---

Copyright 2024 Robert Grimm. The code in this repository has been released as
open source under the [Apache 2.0](LICENSE).
