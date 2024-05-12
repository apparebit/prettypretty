"""Support for serializing and deserializing color values"""
from typing import cast, Literal, NoReturn, overload

from .spec import IntCoordinateSpec, FloatCoordinateSpec


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


def parse_hex(color: str) -> tuple[str, IntCoordinateSpec]:
    entity = 'hex web color'

    try:
        _check(color.startswith('#'), entity, color, 'does not start with "#"')
        color = color[1:]
        digits = len(color)
        _check(digits in (3, 6), entity, color, 'does not have 3 or 6 digits')
        if digits == 3:
            color = ''.join(f'{d}{d}' for d in color)
        return 'rgb256', cast(
            IntCoordinateSpec,
            tuple(int(color[n:n+2], base=16) for n in range(0, 6, 2)),
        )
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)


def parse_x_rgb(color: str) -> tuple[str, FloatCoordinateSpec]:
    entity = 'X rgb color'

    try:
        _check(color.startswith('rgb:'), entity, color, 'does not start with "rgb:"')
        hexes = [(f'{h}{h}' if len(h) == 1 else h) for h in color[4:].split('/')]
        _check(len(hexes) == 3, entity, color, 'does not have three components')
        if max(len(h) for h in hexes) == 2:
            return 'rgb256', cast(
                IntCoordinateSpec,
                tuple(int(h, base=16) for h in hexes),
            )
        else:
            return 'srgb', tuple(int(h, base=16) / 16 ** len(h) for h in hexes)
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)


def parse_x_rgbi(color: str) -> tuple[str, FloatCoordinateSpec]:
    entity = 'X rgbi color'

    try:
        _check(color.startswith('rgbi:'), entity, color, 'does not start with "rgbi:"')
        cs = [float(c) for c in color[5:].split('/')]
        _check(len(cs) == 3, entity, color, 'does not have three components')
        for c in cs:
            _check(0 <= c <= 1, entity, color, 'has non-normal component')
        return 'srgb', cast(FloatCoordinateSpec, tuple(cs))
    except SyntaxError:
        raise
    except:
        _check(False, entity, color)
