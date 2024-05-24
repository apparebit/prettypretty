import enum
from typing import cast

from .color.conversion import get_converter
from .color.lores import rgb6_to_eight_bit
from .color.spec import ColorSpec


class Fidelity(enum.Enum):
    """
    A terminal's color fidelity.

    Attributes:
        NOCOLOR:
        ANSI:
        EIGHT_BIT:
        RGB256:

    When it comes to terminal color support, there are four levels, starting
    with no-color, then the 16 extended ANSI colors, then 8-bit colors, which
    directly incorporate ANSI colors, and finally "truecolor," i.e., RGB256.

    While there is not established mapping between ANSI and RGB256 colors, the
    latter nonetheless subsumes ANSI, notably when it comes to display hardware.
    Historically, no-color was the norm. In fact, even though DEC's VT100 series
    of terminals popularized ANSI escape codes, none of the models in that
    series had color monitors. Nowadays, no-color is important for modelling
    restricted runtime environments, e.g., some continuous integration services,
    as well as user preferences.

    The obvious and motivating use case for fidelity levels is to serve as bound
    that restricts renderable colors. Unless the bound is no-color, color
    formats and spaces outside the bound need to first be converted to *one of
    the formats* within the bound. In almost all cases, that means converting a
    color to the bound's format, i.e., the bound in lower case. Though, with
    RGB256 as bound, RGB6 should be converted to 8-bit, and with ANSI as bound,
    8-bit colors with coordinates -1 through 15 should be converted by changing
    the tag.

    A second use case helps avoid repeated inspections and attempted conversions
    when colors may be shared and reused through style objects. It is based on
    the observation that a fidelity level can also serve as a concise summary of
    past color conversions. Here, :attr:`NOCOLOR` implies a lack of colors and
    no fidelity implies a lack of inspection.

    Fidelity levels form a total order and hence support Python's comparison
    operators.
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

    def is_renderable(self, tag: str) -> bool:
        """
        Determine whether the given color format or space is renderable at this
        terminal fidelity without further preparation including conversion.

        If the color format or space is *not* renderable at this fidelity,
        colors should be converted to the fidelity level, except that RGB6
        should be converted to 8-bit color for RGB256 fidelity.
        """
        if self is Fidelity.NOCOLOR:
            return False
        if tag == 'ansi':
            return True
        if tag == 'eight_bit':
            return self is not Fidelity.ANSI
        if tag == 'rgb256':
            return self is Fidelity.RGB256
        return False

    def prepare_to_render(self, color: None | ColorSpec) -> None | ColorSpec:
        """
        Prepare the color for rendering at this fidelity level. This method
        accepts null colors and, if this fidelity level is :attr:`NOCOLOR`,
        returns null colors.

        This method correctly handles the two corner cases for color
        preparation, converting RGB6 to 8-bit if this fidelity level is RGB256
        and just re-tagging 8-bit colors between -1 and 15 if this fidelity
        level is ANSI.
        """
        if color is None or self is Fidelity.NOCOLOR:
            return None

        if self.is_renderable(color.tag):
            return color

        if color.tag == 'rgb6':
            # Ideally, RGB6 is rendered as 8-bit color
            color = ColorSpec(
                'eight_bit',
                rgb6_to_eight_bit(*cast(tuple[int, ...], color.coordinates))
            )
            if self is not Fidelity.ANSI:
                return color

        if (
            color.tag == 'eight_bit'
            and -1 <= color.coordinates[0] <= 15
            and self is Fidelity.ANSI
        ):
            # Do not relabel for other fidelity levels, SGR parameters differ.
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
