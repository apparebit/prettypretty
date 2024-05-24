"""
Color fidelity.
"""
import enum
from typing import cast

from .color.conversion import get_converter
from .color.lores import rgb6_to_eight_bit
from .color.spec import ColorSpec


class Fidelity(enum.Enum):
    """
    A terminal's color fidelity.

    ANSI escape sequences support three different color formats: ANSI, 8-bit,
    and RGB256, with 8-bit color incorporating ANSI color and RGB256
    incorporating 8-bit color. After adding a "bottom" format *no-color*, we
    have the four color fidelity levels. The primary reason for encapsulating
    them in an enumeration is to simplify comparisons. Otherwise, the
    enumeration constants act much like the corresponding tags for color formats
    or spaces.

    Attributes:
        NOCOLOR: implies *no* color whatsoever;
        ANSI: are the 16+1 extended ANSI colors with text and background defaults;
        EIGHT_BIT: are the 8-bit terminal colors;
        RGB256: is called "truecolor" by many terminals and full 24-bit color.
    """
    NOCOLOR = 0
    ANSI = 1
    EIGHT_BIT = 2
    RGB256 = 3

    @property
    def tag(self) -> str:
        """The corresponding color format or space tag."""
        return self.name.lower()

    @classmethod
    def of(cls, color: None | ColorSpec) -> 'None | Fidelity':
        """
        Determine the fidelity required for the given color specification. If
        the color is ``None``, the required fidelity is ``NOCOLOR``. However, if
        the color has a tag other than ``ansi``, ``rgb6``, ``eight_bit``, or
        ``rgb256``, this method returns ``None``, indicating that no fidelity
        can accommodate the given color.
        """
        if color is None:
            return Fidelity.NOCOLOR
        if color.tag == 'ansi':
            return Fidelity.ANSI
        if color.tag in ('rgb6', 'eight_bit'):
            return Fidelity.EIGHT_BIT
        if color.tag == 'rgb256':
            return Fidelity.RGB256

        return None

    def __lt__(self, other: object) -> bool:
        if not isinstance(other, Fidelity):
            return NotImplemented
        return self.value < other.value

    def __le__(self, other: object) -> bool:
        if not isinstance(other, Fidelity):
            return NotImplemented
        return self.value <= other.value

    def __gt__(self, other: object) -> bool:
        if not isinstance(other, Fidelity):
            return NotImplemented
        return self.value > other.value

    def __ge__(self, other: object) -> bool:
        if not isinstance(other, Fidelity):
            return NotImplemented
        return self.value >= other.value

    def is_renderable(self, color: None | ColorSpec) -> bool:
        """
        Determine whether the color specification is renderable at this terminal
        fidelity.

        For no-color fidelity, *no* color is renderable and *no* conversion can fix
        that. Otherwise, if the color specification is not renderable, then it
        should be converted to the fidelity first, except that RGB6 should be
        converted to 8-bit color for RGB256 fidelity.
        """
        if color is None:
            return True
        if self is Fidelity.NOCOLOR:
            return False
        if color.tag == 'ansi':
            return True
        if color.tag == 'eight_bit':
            return self is not Fidelity.ANSI
        if color.tag == 'rgb256':
            return self is Fidelity.RGB256

        return False

    def prepare_to_render(self, color: None | ColorSpec) -> None | ColorSpec:
        """
        Prepare to render the given color at this terminal fidelity.

        For no-color fidelity, this function just returns ``None``, hence erasing
        all color. If the color specification is ``None`` or renderable at the given
        fidelity, this function just returns the color argument. Otherwise, it
        converts the color specification to the fidelity, except that it converts
        RGB6 to 8-bit colors for RGB256 fidelity.
        """
        if self is Fidelity.NOCOLOR:
            return None

        if self.is_renderable(color):
            return color

        assert color is not None
        if color.tag == 'rgb6':
            # RGB6 is renderable as 8-bit color.
            color = ColorSpec(
                'eight_bit',
                rgb6_to_eight_bit(*cast(tuple[int, ...], color.coordinates))
            )
            if self is not Fidelity.ANSI:
                return color

        elif color.tag == 'eight_bit' and -1 <= color.coordinates[0] <= 15:
            # They might use different SGR parameters...
            return ColorSpec('ansi', color.coordinates)

        return ColorSpec(
            self.tag,
            get_converter(color.tag, self.tag)(*color.coordinates)
        )


# def _defined(*variables: str) -> bool:
#     for variable in variables:
#         if variable in os.environ:
#             return True
#     return False


# def environment_fidelity() -> str:
#     if os.environ.get('TERM') == 'dumb':
#         return 'nocolor'

#     if _defined('CI'):
#         if _defined('GITHUB_ACTIONS', 'GITEA_ACTIONS'):
#             return 'rgb256'

#         if _defined(
#             'TRAVIS', 'CIRCLECI', 'APPVEYOR', 'GITLAB_CI', 'BUILDKITE', 'DRONE'
#         ) or os.environ.get('CI_NAME') == 'codeship':
#             return 'ansi'

#         return 'nocolor'

#     if os.environ.get('COLORTERM') == 'truecolor':
#         return 'rgb256'

#     return 'eight_bit'
