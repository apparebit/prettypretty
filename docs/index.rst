Prettypretty Terminals
======================


Prettypretty helps build terminal user interfaces in Python. Notably, it
provides expressive color management, including operating in perceptually
uniform color spaces.

.. toctree::
   :hidden:

   self

.. toctree::
   :caption: Background

   formats-and-spaces
   conversions

.. toctree::
   :caption: API

   apidocs/color
   apidocs/grid
   apidocs/style

.. toctree::
   :caption: Links
   :hidden:

   Repository <https://github.com/apparebit/prettypretty>


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
