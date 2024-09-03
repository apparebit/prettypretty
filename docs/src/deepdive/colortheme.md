# Judgement Day: Color Themes

> De gustibus non disputandum est.

There is no accounting for taste. Or so the ancient Roman adage maintains.


## Matters of Taste

Like so many others, the above adage holds a kernel of Truth, and it does serve
as useful reminder to bow out of discussions that can only lead to strive. Like
so many others, the above adage also avoids answering the hard questions. Above
all, in this case, the hard questions are:

> What are matters of taste?
> What are matters of accountability?

Most of us probably lean towards treating color themes as matters of taste. I
certainly do. The sheer number of color themes [available on the
web](https://gogh-co.github.io/Gogh/) also supports that contention. Alas, most
of us who regularly use terminals probably also have strongly held convictions
about the one true color theme. I certainly do as well. While that might at
first appear as a contradiction or case of cognitive dissonance, it distinctly
is not: The first of my certainties is about a large group of people. The second
is about myself.

The lesson for application developers should be clear:

> Respect your users' preferences and accommodate them with little to no effort
> on their part!

It's worth pointing out that those user preferences may vary even for the same
person on the same day, depending on task or context. For instance, I personally
do not care for dark mode *at all*. When I am focused on text, dark mode is
quite uncomfortable to me and, on bad days, almost becomes a migraine trigger.
Except, when I am working on graphics or photographs, I *strongly* prefer the
editor to have a dark and subdued user interface and usually have it cover
(almost) my entire screen.

That same respect for users's preferences is a major motivation for prettypretty
and very much informs its design. That's why prettypretty queries the terminal
for the current color theme. That's why it models terminal capabilities *as well
as* user preferences. That's why it provides high-quality translation down to
8-bit and ANSI colors—well, one of the reasons for providing high-quality color
translation.


## Matters of Accountability

Something funny just happened. I was discussing matters of taste and then
started accounting for the ways prettypretty helps terminal applications
accommodate their users. In other words, knowing what are matters of taste and
how to accommodate them may very well be a matter of accountability. In fact,
the open source ecosystem is holding tools accountable for matters of taste, to
wit [`NO_COLOR`](https://no-color.org) and
[`FORCE_COLOR`](https://force-color.org) advocating the use of the eponymous
environment variables to keep their terminals colorfree and rainbow-colored,
respectively.

Since color themes triggered me into being accountable, I am now wondering
whether color themes themselves are a question of taste or if there are in fact
objective criteria for evaluating the quality of color themes. I'd say that's
worth exploring.

So, I decided to visualize color themes not primarily as colors but as color
coordinates. For a plot of color coordinates to be meaningful, I needed a
perceptually uniform color space. Oh, Oklab! I also wanted something simpler
than 3D visualizations. Well, I am interested in *color*. So it seems acceptable
to drop lightness and just plot a/b or preferably chroma and hue. That worked
for three or so weeks, until I realized that I need to consider lightness as
well. Still, no 3D. Only a second, smaller graph that illustrats the third
component. So without further ado, here we are:

### Apple's "Basic" Theme

<div align=center>

![the colors in Apple's "Basic" theme for Terminal.app on the chroma/hue plane
of Oklab](colortheme/terminal.app-colors.svg)

</div>

The above graphic illustrates Apple's "Basic" theme for the macOS Terminal.app.
The larger plot on top plots the 12 theme colors as well as the 4 theme grays,
hence the "12 + 4" in the title, on the chroma/hue plane of the perceptually
uniform [Oklab](https://bottosson.github.io/posts/oklab/). Since the reduction
in dimensionality collapses all grays onto the origin, the single marker takes
on the average lightness of the four grays. In addition to the 12+4 colors, the
larger plot on top also shows the boundary of the sRGB gamut. Meanwhile, the
smaller plot on bottom is a bar graph for the [revised lightness
Lr](https://bottosson.github.io/posts/colorpicker/#intermission---a-new-lightness-estimate-for-oklab)
of the 12 theme colors. Technically, that makes the color space Oklrch. But the
chroma/hue plane of Oklrch is identical to the chroma/hue plane of Oklch, the
a/b plane of Oklrab, and the a/b plane of Oklab.

Ok? Oklab!

What can we learn from this graph? Quite a bit:

  * The regular and bright color values belonging to the same pair are clearly
    distinguished from each other by lightness and by chroma.
  * However, with exception of cyan, all pairs share the same hue.
  * All bright colors, without exception, have the same hue as one of the sRGB
    primaries and secondaries.
  * In fact, bright blue is identical to the sRGB primary. The other five bright
    colors are distinct from the sRGB primaries and secondaries, but they too
    have high chroma values.

Because of sRGB's gamut has very limited coverage of cyan, finding a regular
cyan with the same hue as the bright cyan and the sRGB cyan secondary that also
is clearly distinguishable from the bright cyan by lightness and chroma would
seem like a tall order. So the divergence in hue seems like an acceptable
compromise for having two fairly saturated colors with a substantial difference
in lightness and chroma.

In summary, it seems safe to say that the basic terminal theme is a lively and
saturated celebration of all things sRGB, whose hues dominate the color theme.
At the same time, the colors are carefully differentiated in both lightness and
chroma, which helps people with less than perfect color vision. It also isn't
gaudy: Only two colors have a lightness above 0.75 and only one color is
identical to a primary or secondary. That too seems reasonable, since the
primary in question is blue, i.e., the part of the visual spectrum that is
detected by over an order of magnitude fewer cells than the greener and redder
frequencies.


### iTerm2's "Light Background" Theme

For comparison, here is the "light background" theme for
[iTerm2](https://iterm2.com):

<div align=center>

![the colors in iTerm2's "light" theme on the chroma/hue plane of
Oklab](colortheme/iterm-colors.svg)

</div>

iTerm2 is a fantastic open source terminal emulator. *Really!* But as you can
see above, its light background color theme might benefit from some
improvements. Notably, it is overly reliant on lightness differences to separate
theme colors from each other. In fact, the yellow, green, cyan, and magenta
pairs lack differentiation in both chroma and hue. The only two pairs that dare
to stick out are red and blue.

On the taste side of things, I appreciate that its colors are brighter and less
intense. That makes for a lighter, airier feeling and beautiful colors. The
question is: Can they be separated along two axes without compromising the
overall quality of the theme?


### The "Visual Studio Light" Theme

As the third (and final) example, here is the "Visual Studio Light" theme for VS
Code:

<div align=center>

![the colors in VS Code's "Visual Studio Light" theme on the chroma/hue plane of
Oklab](colortheme/vscode-colors.svg)

</div>

Huh? 8+4 colors? What's going on?

Yup. I feel cheated out of colors, too. Especially since there are only 16 ANSI
colors to begin with. If we were reviewing 24-bit sRGB colors, we might not
notice if four out of 16,777,216 colors went MIA. But when a quarter of colors
go MIA, we notice. And not in a good way.

Well, technically, those four colors are still there. It's just that the
nonbright and bright versions of red, cyan, blue, and magenta each have the same
color values.

But it doesn't stop there: The two yellows are pretty darn close to each other
as well. It seems only a question of time before they also collapse into
another. The only nicely differentiated colors are the greens.

Why did the theme designer do this? Anyone in the know?

Clearly, we are back to matters of taste. And there simply is no accounting for
taste. Or the total lack of it. 😳

<center style="margin: 2em 0">⁂</center>

The above figures were all generated with `prettypretty.plot`. While I
originally set out to plot color themes only, the script was too useful to be
restricted that way. Hence, you can plot arbitrary colors listed in a text file,
too. As many or as few as you want. `plot` automatically adjusts the "zoom
factor" along the chroma axis between 0.3 and 0.6. But you decide whether to
show gamut boundaries and, if so, for what color spaces. sRGB, Display P3, and
Rec. 2020. Take your pick.

`plot` 'em colors. `plot` 'em real good! 🤪
