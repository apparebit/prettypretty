Pretty 🌸 Pretty
================

This is the entry page for the part of the documentation that focuses on
prettypretty's pythonic side. Unfortunately, however, there are no tools to
generate documentation for Python extension modules. So there also is no
Python-specific documentation for what's clearly pretttypretty's most important
module, ``prettypretty.color``. For now, please make do with the `type stub
<https://github.com/apparebit/prettypretty/blob/main/prettypretty/color/__init__.pyi>`_,
which should reassure you that most classes are just like enums, as well as the
`documentation for prettypretty's Rust code
<https://apparebit.github.io/prettypretty/>`_, which is comprehensive and
includes even methods that are available in Python only. Also, you can pretty
much ignore traits and lifetimes, since they have no relevance for code using
Python extension modules from Python.


.. toctree::
   :maxdepth: 1
   :caption: Modules

   prettypretty/ansi
   prettypretty/darkmode
   prettypretty/ident
   prettypretty/style_extras
   prettypretty/terminal
   prettypretty/theme

.. toctree::
   :maxdepth: 1
   :hidden:
   :caption: Links

   Documentation Home <https://apparebit.github.io/prettypretty/>
   Repository <https://github.com/apparebit/prettypretty/>
