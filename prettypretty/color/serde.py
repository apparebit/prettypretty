"""Support for serializing and deserializing color values"""
import enum
from typing import cast, Literal, NoReturn, overload


@overload
def _check(
    is_valid: Literal[False], entity: str, value: object, deficiency: str = ...
) -> NoReturn:
    ...
@overload
def _check(
    is_valid: bool, entity: str, value: object, deficiency: str = ...
) -> None | NoReturn:
    ...
def _check(
    is_valid: bool, entity: str, value: object, deficiency: str = 'is malformed'
) -> None | NoReturn:
    if not is_valid:
        raise SyntaxError(f'{entity} "{value}" {deficiency}')
    return


def parse_hex(color: str) -> tuple[str, tuple[int, int, int]]:
    """Parse the string specifying a color in hashed hexadecimal format."""
    entity = 'hex web color'

    try:
        _check(color.startswith('#'), entity, color, 'does not start with "#"')
        color = color[1:]
        digits = len(color)
        _check(digits in (3, 6), entity, color, 'does not have 3 or 6 digits')
        if digits == 3:
            color = ''.join(f'{d}{d}' for d in color)
        return 'rgb256', cast(
            tuple[int, int, int],
            tuple(int(color[n:n+2], base=16) for n in range(0, 6, 2)),
        )
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)


def parse_x_rgb(color: str) -> tuple[str, tuple[float, float, float]]:
    """Parse the string specifying a color in X's rgb: format."""
    entity = 'X rgb color'

    try:
        _check(color.startswith('rgb:'), entity, color, 'does not start with "rgb:"')
        hexes = [(f'{h}{h}' if len(h) == 1 else h) for h in color[4:].split('/')]
        _check(len(hexes) == 3, entity, color, 'does not have three components')
        if max(len(h) for h in hexes) == 2:
            return 'rgb256', cast(
                tuple[int, int, int],
                tuple(int(h, base=16) for h in hexes)
            )
        else:
            return 'srgb', tuple(int(h, base=16) / 16 ** len(h) for h in hexes)
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)


def parse_x_rgbi(color: str) -> tuple[str, tuple[float, float, float]]:
    """Parse the string specifying a color in X's rgbi: format."""
    entity = 'X rgbi color'

    try:
        _check(color.startswith('rgbi:'), entity, color, 'does not start with "rgbi:"')
        cs = [float(c) for c in color[5:].split('/')]
        _check(len(cs) == 3, entity, color, 'does not have three components')
        for c in cs:
            _check(0 <= c <= 1, entity, color, 'has non-normal component')
        return 'srgb', cast(tuple[float, float, float], tuple(cs))
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)


class Format(enum.Enum):
    """
    The color format

    Attributes:
        FUNCTION: for ``<tag>(<coordinates>)`` notation
        HEX: for ``#<hex>`` notation
        CSS: for ``color()``, ``oklab()``, ``oklch()``, and ``rgb()`` notation
        X: for ``rgb:<hex>/<hex>/<hex>`` and ``rgbi:<float>/<float>/<float>``
            notation
    """
    FUNCTION = 'f'
    HEX = 'h'
    CSS = 's'
    X = 'x'


def parse_format_spec(spec: str) -> tuple[Format, int]:
    """
    Parse the color format specifier into the format and precision.

    Args:
        spec: selects the desired output format and precision
    Returns:
        the format and maximum precision for floating point numbers, which
        default to `Format.FUNCTION` and 5, respectively

    A valid format specifier comprises two parts, both of which are optional:

     1. The first part, if present specifies the precision and is written as a
        period followed by one or two decimal digits, e.g., ``.3``.
     2. The second part, if present, specifies the format:

          * ``f`` for function notation, which uses the tag as function name
            and the comma-separated coordinates as arguments
          * ``h`` for hexadecimal notation prefixed with a hash ``#``
          * ``s`` for CSS notation, which uses CSS function and color space
            names and space-separated coordinates
          * ``x`` for X notation, which uses ``rgb:`` or ``rgbi:`` as a prefix

    Note that the terminal-specific ``ansi``, ``eight_bit``, and ``rgb6`` color
    formats as well as the ``linear_p3`` color space have no CSS serialization.
    Also note that ``h`` only works for RGB256 colors and ``x`` only for RGB256
    and sRGB colors.
    """
    format = Format.FUNCTION
    precision = 5

    s = spec
    if s:
        f = s[-1]
        if f in ('f', 'h', 's', 'x'):
            format = Format(f)
            s = s[:-1]
    if s.startswith('.'):
        precision = int(s[1:])
        s = ''
    if s:
        raise ValueError(f'malformed color format "{spec}"')

    return format, precision


_CSS_FORMATS = {
    'rgb256': 'rgb({})',
    'srgb': 'color(srgb {})',
    'linear_srgb': 'color(srgb-linear {})',
    'p3': 'color(display-p3 {})',
    'xyz': 'color(xyz {})',
    'oklab': 'oklab({})',
    'oklch': 'oklch({})',
}

def stringify(
    tag: str,
    coordinates: tuple[int] | tuple[float, float, float],
    format: Format = Format.FUNCTION,
    precision: int = 5
) -> str:
    """
    Format the tagged coordinates in the specified format and with the specified
    precision.
    """
    if format is Format.HEX:
        return '#' + ''.join(f'{c:02x}' for c in coordinates)
    elif format is Format.X:
        if all(isinstance(c, int) for c in coordinates):
            return 'rgb:' + '/'.join(f'{c:02x}' for c in coordinates)
        else:
            return 'rgbi:' + '/'.join(f'{float(c):.{precision}}' for c in coordinates)

    separator = ' ' if format is Format.CSS else ', '
    coordinate_text = separator.join(
        f'{c}' if isinstance(c, int) else f'{c:.{precision}}'
        for c in coordinates
    )

    if format is Format.FUNCTION:
        return f'{tag}({coordinate_text})'

    css_format = _CSS_FORMATS.get(tag)
    if css_format is None:
        raise ValueError(f'{tag} has no CSS serialization')
    return css_format.format(coordinate_text)
