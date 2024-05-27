import enum
import os
from typing import cast, Literal, Self, TypeAlias

from .color.conversion import get_converter, srgb_to_rgb256
from .color.gamut import map_into_gamut
from .color.lores import rgb6_to_eight_bit
from .color.spec import ColorSpec


FidelityTag: TypeAlias = Literal['plain', 'nocolor', 'ansi', 'eight_bit', 'rgb256']

class Fidelity(enum.Enum):
    """
    A terminal's color fidelity.

    Attributes:
        PLAIN:
        NOCOLOR:
        ANSI:
        EIGHT_BIT:
        RGB256:

    When it comes to terminal color support, there are four levels, starting
    with no-color, then the 16 extended ANSI colors, then 8-bit colors, which
    directly incorporate ANSI colors, and finally "truecolor," i.e., RGB256.
    Since no-color still allows for styling text with ANSI escape codes,
    fidelity also includes the plain level, which prohibits the use of escapes.

    Even though there is no established mapping between ANSI and RGB256 colors,
    the latter nonetheless subsumes ANSI, most certainly when it comes to
    display hardware. Historically, no-color was the norm for terminals. In
    fact, even though DEC's VT100 series of terminals popularized ANSI escape
    codes, none of the models in that series had color monitors. Nowadays,
    no-color is important for modelling restricted runtime environments, e.g.,
    as found on continuous integration services. Meanwhile plain is important
    when producing machine-readable output. Furthermore, both no-color and plain
    can capture user preferences.

    The original use case for fidelity levels is serving as bounds that restrict
    renderable colors—and renderable styles in case of the plain level. Unless
    the bound is plain or no-color, color formats and spaces outside the bound
    need to be converted to *one of the formats* within the bound. In almost all
    cases, that means converting a color to the bound's format (denoted by the
    bound's name in lower case).

    However, there are three complications.

     1. When the fidelity level is RGB256, the original color may very well be
        outside of RGB256's gamut as well. Since gamut mapping is more accurate
        with floating point coordinates, preparing colors for RGB256 requires
        first conversion to sRGB, then gamut mapping, and finally conversion to
        RGB256.
     2. Since RGB6 is a three-component version of (part of) 8-bit color,
        converting to 8-bit color suffices for fidelity levels of 8-bit and
        RGB256. In the latter case, 8-bit color may not be the bound's format
        but it certainly is within the bound. Converting RGB6 to 8-bit is a
        reasonable first step for ANSI fidelity, too.
     3. When the fidelity level is ANSI and the original color is 8-bit with a
        component value between -1 and 15 inclusive, a trivial re-tagging of the
        color suffices. Technically, -1 stands for the default color. But that
        color is part of the core ANSI colors, too.

    A second use case helps avoid repeated inspections and attempted conversions
    when colors may be shared and reused through style objects. It is based on
    the observation that a fidelity level can also serve as a concise summary of
    past color conversions. Here, :attr:`NOCOLOR` implies a lack of colors and
    no fidelity implies a lack of inspection.

    Fidelity levels form a total order and support Python's comparison
    operators.
    """
    PLAIN = -1
    NOCOLOR = 0
    ANSI = 1
    EIGHT_BIT = 2
    RGB256 = 3

    @property
    def tag(self) -> str:
        """The corresponding color format or space tag."""
        return self.name.lower()

    @classmethod
    def from_tag(cls, tag: FidelityTag | Self) -> Self:
        """Instantiate the fidelity level from a tag."""
        if isinstance(tag, cls):
            return tag

        assert isinstance(tag, str)
        try:
            return cls[tag.upper()]
        except KeyError:
            return cls[tag]

    @classmethod
    def from_color(cls, color: None | ColorSpec) -> 'None | Fidelity':
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
        if self in (Fidelity.PLAIN, Fidelity.NOCOLOR):
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

        This method correctly handles the three corner cases for color
        preparation. First, independent of fidelity level, it converts RGB6 to
        8-bit before considering further conversions. Second, instead of
        directly converting to RGB256, it first converts to sRGB, then
        gamut-maps the result, and only then converts to RGB256. Finally, when
        converting to ANSI, it simply re-tags 8-bit colors between -1 and 15.
        """
        if color is None or self in (Fidelity.PLAIN, Fidelity.NOCOLOR):
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

        if self is Fidelity.RGB256:
            # Ensure color is in sRGB gamut before converting into RGB256
            coordinates = get_converter(color.tag, 'srgb')(*color.coordinates)
            coordinates = map_into_gamut('srgb', coordinates)
            return ColorSpec('rgb256', srgb_to_rgb256(*coordinates))

        return ColorSpec(
            self.tag,
            get_converter(color.tag, self.tag)(*color.coordinates)
        )


def _defined(*variables: str) -> bool:
    for variable in variables:
        if variable in os.environ:
            return True
    return False


def environment_fidelity(is_tty: bool) -> Fidelity:
    """
    Determine the current terminal's fidelity based on the environment variables
    for this process. This function incorporates logic from the `supports-color
    <https://github.com/chalk/supports-color/blob/main/index.js>`_ package. The
    ``istty`` argument indicates whether the terminal's output is a TTY.
    """
    force = os.environ.get('FORCE_COLOR')
    if force is not None and force != 'false':
        if force in ('', '1', 'true'):
            return Fidelity.ANSI
        if force == '2':
            return Fidelity.EIGHT_BIT
        if force == '3':
            return Fidelity.RGB256

    if _defined('TF_BUILD', 'AGENT_NAME'):
        # Azure DevOps
        return Fidelity.ANSI

    if not is_tty:
        return Fidelity.NOCOLOR

    TERM = os.environ.get('TERM')

    if TERM == 'dumb':
        return Fidelity.NOCOLOR

    # FIXME: Windows 10 build 10586 for eight_bit,
    # Windows 10 build 14931 for truecolor, but what terminal??

    if _defined('CI'):
        if _defined('GITHUB_ACTIONS', 'GITEA_ACTIONS'):
            return Fidelity.RGB256

        if _defined(
            'TRAVIS', 'CIRCLECI', 'APPVEYOR', 'GITLAB_CI', 'BUILDKITE', 'DRONE'
        ) or os.environ.get('CI_NAME') == 'codeship':
            return Fidelity.ANSI

        return Fidelity.NOCOLOR

    if os.environ.get('COLORTERM') == 'truecolor':
        return Fidelity.RGB256

    if TERM == 'xterm-kitty':
        return Fidelity.RGB256

    if TERM and (TERM.endswith('-256') or TERM.endswith('-256color')):
        return Fidelity.EIGHT_BIT

    # Even the Windows CMD shell does the basic colors
    return Fidelity.ANSI
