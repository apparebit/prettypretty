prettypretty.color
==================

This subpackage includes prettypretty's low-level and high-level color APIs.
Much of the functionality of the low-level API is implemented through simple
functions and tuples, which are spread out across modules besides
``prettypretty.color.object`` and straightforward to reuse and compose.
Different modules also are largely independent from each other. The one
exception is :mod:`prettypretty.color.lores`, which implements support for
low-resolution terminal colors with help of the
:mod:`prettypretty.color.conversion`, :mod:`prettypretty.color.difference`, and
:mod:`prettypretty.color.theme` modules.

The high-level API is a single class, :mod:`.Color`, which is exported by the
``prettypretty.color.object`` module and provides access to the same
functionality as the low-level API, including the conversion between arbitrary
color spaces. The price for both convenience and encapsulation is higher
overhead, strictly more code is executed, and the potential for less accurate
results, more floating point operations may be executed.


prettypretty.color.apca
-----------------------

.. automodule:: prettypretty.color.apca
    :members:


prettypretty.color.conversion
-----------------------------

.. automodule:: prettypretty.color.conversion
    :members:


prettypretty.color.difference
-----------------------------

.. automodule:: prettypretty.color.difference
    :members:


prettypretty.color.equality
---------------------------

.. automodule:: prettypretty.color.equality
    :members:


prettypretty.color.lores
------------------------

.. automodule:: prettypretty.color.lores
    :members:


prettypretty.color.object
-------------------------

.. automodule:: prettypretty.color.object
    :members:


prettypretty.color.serde
------------------------

.. automodule:: prettypretty.color.serde
    :members:


prettypretty.color.space
------------------------

.. automodule:: prettypretty.color.space
    :members:


prettypretty.color.spec
-----------------------

.. automodule:: prettypretty.color.spec
    :members:


prettypretty.color.style
------------------------

.. automodule:: prettypretty.color.style
    :members:


prettypretty.color.theme
------------------------

.. automodule:: prettypretty.color.theme
    :members:
