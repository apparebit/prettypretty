"""Utility module for traversing the boundary of a color space's gamut."""

from typing import ClassVar, Self

from . import Color

class GamutTraversalStep_MoveTo(GamutTraversalStep):
    """Start new path by moving to color coordinates."""


class GamutTraversalStep_LineTo(GamutTraversalStep):
    """Continue path by drawing line to color coordinates."""


class GamutTraversalStep_CloseWith(GamutTraversalStep):
    """Close path by drawing line to color coordinates."""


class GamutTraversalStep:
    """A step along a path during gamut boundary traversal."""
    MoveTo: ClassVar[type[GamutTraversalStep]] = GamutTraversalStep_MoveTo
    LineTo: ClassVar[type[GamutTraversalStep]] = GamutTraversalStep_LineTo
    CloseWith: ClassVar[type[GamutTraversalStep]] = GamutTraversalStep_CloseWith

    def color(self) -> Color: ...
    def __repr__(self) -> str: ...


class GamutTraversal:
    """An iterator over RGB gamut boundaries."""
    def __iter__(self) -> Self: ...
    def __next__(self) -> GamutTraversalStep: ...
    def __repr__(self) -> str: ...
