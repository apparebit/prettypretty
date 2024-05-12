prettypretty
============

Prettypretty helps build terminal user interfaces in Python. Notably, it
provides expressive color management, including operating in perceptually
uniform color spaces.

.. toctree::
   :maxdepth: 1
   :hidden:

   self

.. toctree::
   :maxdepth: 1
   :caption: Background

   formats-and-spaces
   conversions

.. toctree::
   :maxdepth: 1
   :caption: API

   apidocs/color
   apidocs/grid
   apidocs/style

.. toctree::
   :maxdepth: 1
   :caption: Links
   :hidden:

   Repository <https://github.com/apparebit/prettypretty>


Getting Started
---------------

As usual, you need to install prettypretty first:

.. code-block:: sh

   $ pip install prettypretty


Once prettypretty is installed, you can start using its API in your code.

Prettypretty's color support is exposed through two APIs. The low-level API is
almost entirely procedural and correspondingly lightweight and flexible. But it
can also be a bit unwieldy. Meanwhile, the high-level API exposes the same
functionality, albeit at some performance overhead, through a single class,
:class:`prettypretty.color.object.Color`. However, since both APIs use the same
tags to identify color formats and spaces and use the same tuple-based
representation for color coordinates, it's easy to exchange colors between the
two.


Acknowledgements
----------------

Implementing this package's color support was a breeze. In part, that was
because I had built a prototype before and knew what I was going for. In part,
that was because I copied many of the nitty-gritty color algorithms and
conversion matrices from the most excellent `Color.js <https://colorjs.io>`_
library by `Lea Verou <http://lea.verou.me/>`_ and `Chris Lilley
<https://svgees.us/>`_. Theirs being a JavaScript library and mine being a
Python package, there are many differences, small and not so small. But without
Color.js, I could not have implemented color support in less than a week. Thank
you!
