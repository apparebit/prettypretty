How to Pretty-Pretty: Style
===========================

Let's see what prettypretty can do for your command line tools. I picked the
implementation of a progress bar for several reasons. First, I've been itching
to write one myself for quite a while now. Second, animated anything is nice and
shiny, i.e., makes for a good demo. Third, the script is simple enough to fit
into less than 100 lines of Python, yet complex enough to show off most major
features. The `complete script
<https://github.com/apparebit/prettypretty/blob/main/prettypretty/progress.py>`_
is part of prettypretty's distribution.

*How to Pretty-Pretty* has two parts. This part focuses on prettypretty-specific
code in the progress bar script. The other part focuses on prettypretty's color
support. That includes a discussion of how prettypretty adjusts colors to
terminal capabilities and plenty of examples for manipulating colors. You
probably want to start by working through this part to get a good overview. But
if your learning styles favors fundamentals first, then the other part probably
is a better start.


Visualizing Progress
--------------------

In either case, you probably want to get started by running the progress bar
script yourself. So please go through the usual incantations for creating a new
virtual environment, installing prettypretty into it, and running the progress
bar demo:

.. code-block:: console

   $ mkdir progress
   $ cd progress
   $ python -m venv .venv
   $ source .venv/bin/activate
   $ python -m pip install prettypretty
   Downloading prettypretty-0.3.0-py3-none-any.whl (64 kB)
   Installing collected packages: prettypretty
   Successfully installed prettypretty-0.3.0
   $ python -m prettypretty.progress

(The above command line incantations work just as written on Linux and macOS.
But you may have to adjust them somewhat, if you use a package manager other
than pip or are running Windows. I trust you know what to do differently.)

That last command actually executes the progress bar script. You should see a
green bar rapidly go from 0% to 100%. It may end up looking like this:

.. image:: figures/progress-bar-light.png
   :alt: A bright green progress bar at 100% against a white background
   :scale: 50 %

Or, if your terminal's color theme is a dark theme, it may end up looking more
like this:

.. image:: figures/progress-bar-dark.png
   :alt: A medium green progress bar at 100% against a black background
   :scale: 50 %


Of Styles and Terminals
-----------------------

In addition to colors, terminals support a few more attributes for styling text,
including to change its weight or slant, add over-, middle-, and underlines, and
so on. While you can directly write styles to terminal output, the more robust
alternative is to declare all of your application's styles in one place. That
helps with the design and maintenance of styles. It also makes it easier to
select the right styles for use. The progress bar script uses the :class:`.rich`
builder to declare two styles:

.. code-block:: python

   LIGHT_MODE_BAR = rich().fg('p3', 0, 1, 0).style()
   DARK_MODE_BAR = rich().fg('rgb256', 3, 151, 49).style()

You could create styles with their constructor but, depending on the number of
attributes you'd like set, that won't be very readable. The :class:`rich`
builder is the cleaner option, since it provides a fluent interface for
declaring styles—and also rich text sequences, which increase the repertoire
with hyperlinks and cursor movement. Let's stick with styles, however, and
see how we might declare a warning style:

.. code-block:: python

   WARNING = rich().bold.fg(0).bg(220).style()

Compared to the two styles above, the warning style also sets a text attribute
(through a property) and the background color (through a method). The latter
color, by the way, is a bright 8-bit orange. When declaring a style, only
include attributes that you want set and nothing else. Also, don't bother with
defining styles that undo other styles or incrementally transition from one
style to another.

You can easily and automatically compute them with Python's negation ``~``,
subtraction ``-``, and alternative ``|`` operators. In particular, the style
``~style`` undoes all attributes of ``style``, hence restoring the terminal to
its default appearance. The style ``style2 - style1`` incrementally transitions
from ``style1`` to ``style2``. The style ``style1 | style2`` lets you recover
complete styles from incremental transitions by combining all attributes from
``style1`` and ``style2``. Remember that ``|`` does give precedence to the first
operand ``style1`` if both of them impact the same attribute.

For example, `the last line
<https://github.com/apparebit/prettypretty/blob/da0d1a6d0277dd3a240a1b49037925036f7e8498/prettypretty/progress.py#L55>`_
of the ``format_bar`` function in the progress bar script uses negation for its
intended purpose, restoring the default appearance:

.. code-block:: python

   return RichText.of('  ┫', style, bar, ~style, '┣', f' {percent:5.1f}%')

:class:`RichText` is a sequence of strings, styles, and so on that simplifies
color adjustment during output. You don't need to use it but it may speed up
output a little bit.

The progress bar script's `main function
<https://github.com/apparebit/prettypretty/blob/da0d1a6d0277dd3a240a1b49037925036f7e8498/prettypretty/progress.py#L67>`_
illustrates how to go from style declarations to usable styles and how to
display the resulting rich text. It starts out by creating a terminal object,
possibly overwriting its color fidelity, querying the terminal for its current
color scheme, hiding the cursor, and scoping all styles. The ``with`` block
ensures that the cursor reappears and no custom style leaks into your terminal
even if the application raises an exception.

.. code-block:: python

    with (
        Terminal(fidelity=options.fidelity)
        .terminal_theme()
        .hidden_cursor()
        .scoped_style()
    ) as term:

I strongly recommend to always scope styles in a ``with`` statement. In all
likelihood, you also want to read the current terminal theme. That's the
one-line price of admission for prettypretty. It might be possible to fold the
theme query into :class:`.Terminal`'s constructor. But that query fails if the
input is being redirected. It also involves quite a bit of I/O, since it writes
18 ANSI escape sequences to the terminal and parses 18 ANSI escape sequences as
responses from the terminal. Consequently, making this operation an explicit one
seems the better interface design.

Prettypretty supports several more contextual operations, including for updating
the :meth:`.Terminal.window_title`, using the
:meth:`.Terminal.alternate_screen`, performing :meth:`.Terminal.batched_output`,
and enabling :meth:`.Terminal.bracketed_paste`. You can perform them
individually, each in its own ``with`` statement, or you can fluently combine
them with each other in a single ``with`` statement similar to the above
example.

Once the terminal has been set up, the progress bar script uses
``is_dark_theme`` to pick the right style and adjusts the style to the
terminal's :attr:`.Terminal.fidelity`:

.. code-block:: python

   style = DARK_MODE_BAR if is_dark_theme() else LIGHT_MODE_BAR
   style = style.prepare(term.fidelity)

Doing so once during startup avoids the not insubstantial overhead of color
conversion on the critical path.

With that, the progress bar script is ready for turning progress reports into
progress bar updates. Each update assembles the rich text for the progress bar,
moves the (invisible) cursor to the beginning of the line, writes the rich text
to terminal output, and flushes the output.

.. code-block:: python

    for percent in progress_reports():
        bar = format_bar(percent, style)
        term.column(0).rich_text(bar).flush()
        time.sleep(random.uniform(1/60, 1/10))

After accomplishing so much with so little code, our progress bar script
deserves some rest and so it sleeps for a spell.

Much of the rest of the progress bar script is not specific to prettypretty. Its
line breakdown looks like this:

+------------------+---------------+
| Function         | Lines of Code |
+==================+===============+
| Imports          | 9             |
+------------------+---------------+
| Argument parser  | 18            |
+------------------+---------------+
| Module constants | 7             |
+------------------+---------------+
| Progress bar     | 14            |
+------------------+---------------+
| Progress reports | 7             |
+------------------+---------------+
| main()           | 23            |
+------------------+---------------+
| Calling main()   | 2             |
+------------------+---------------+
| *Total*          | *80*          |
+------------------+---------------+

Note that only one line out of 14 for formatting the progress bar is specific to
prettypretty. Likewise, it takes only one line in ``main()`` to write out the
progress bar. Startup is somewhat more hefty, comprising 8 prettypretty-specific
lines of code. Then again, 6 of them are very generously spaced.

Happy, happy, joy, joy!
