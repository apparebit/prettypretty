import os

from .color import Fidelity


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
            return Fidelity.Ansi
        if force == '2':
            return Fidelity.EightBit
        if force == '3':
            return Fidelity.Full

    if _defined('TF_BUILD', 'AGENT_NAME'):
        # Azure DevOps
        return Fidelity.Ansi

    if not is_tty:
        return Fidelity.Plain

    TERM = os.environ.get('TERM')

    if TERM == 'dumb':
        return Fidelity.Plain

    # FIXME: Windows 10 build 10586 for eight_bit,
    # Windows 10 build 14931 for truecolor, but what terminal??

    if _defined('CI'):
        if _defined('GITHUB_ACTIONS', 'GITEA_ACTIONS'):
            return Fidelity.Full

        if _defined(
            'TRAVIS', 'CIRCLECI', 'APPVEYOR', 'GITLAB_CI', 'BUILDKITE', 'DRONE'
        ) or os.environ.get('CI_NAME') == 'codeship':
            return Fidelity.Ansi

        return Fidelity.Plain

    if os.environ.get('COLORTERM') == 'truecolor':
        return Fidelity.Full

    if TERM == 'xterm-kitty':
        return Fidelity.Full

    if TERM and (TERM.endswith('-256') or TERM.endswith('-256color')):
        return Fidelity.EightBit

    # Even the Windows CMD shell does the basic colors
    return Fidelity.Ansi
