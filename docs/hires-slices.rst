Hi-Res Slices
=============

To illustrate how prettypretty fares with arbitrary colors being downsampled to
8-bit terminal colors, here are three slices through the RGB cube along two axes
each while holding the third axis fixed at either 0 or 255. For each such
configuration, there are two screenshots, one to show off the grid with full
color and one to illustrate the reduction to 8-bit. Since a 256x256 grid would
be a tad large, the stride for the two non-constant colors is 8.

The conversion down to 8-bit color only considers the 6x6x6 RGB cube and 24-step
gray gradient but ignores the 16 extended ANSI colors. I did include them
originally as well. But when generating color grids, I noticed that the extended
ANSI colors are rather distinct from the other 8-bit colors. Notably, they don't
fit neatly onto any color graduations and thus are bound to stick out. Similar
to web designers who went looking for different color spaces after trying to
form visually satisfying gradients in sRGB, I found that gradual color
transitions are a great way for testing automatic color manipulation because
they make outliers easily noticeable.


Slicing Through GB
------------------

GB With R=0
^^^^^^^^^^^

.. image:: figures/slice-r00.png
   :alt: with


.. image:: figures/slice-r00-reduced.png
   :alt: with


GB With R=255
^^^^^^^^^^^^^

.. image:: figures/slice-rff.png
   :alt: with


.. image:: figures/slice-rff-reduced.png
   :alt: with


Slicing Through RB
------------------

RB With G=0
^^^^^^^^^^^

.. image:: figures/slice-g00.png
   :alt: with


.. image:: figures/slice-g00-reduced.png
   :alt: with


RB With G=255
^^^^^^^^^^^^^

.. image:: figures/slice-gff.png
   :alt: with


.. image:: figures/slice-gff-reduced.png
   :alt: with


Slicing Through RG
------------------

RG With B=0
^^^^^^^^^^^

.. image:: figures/slice-b00.png
   :alt: with


.. image:: figures/slice-b00-reduced.png
   :alt: with


RG With B=255
^^^^^^^^^^^^^

.. image:: figures/slice-bff.png
   :alt: with


.. image:: figures/slice-bff-reduced.png
   :alt: with


