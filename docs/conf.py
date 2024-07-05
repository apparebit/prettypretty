# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Path setup --------------------------------------------------------------
# If extensions (or modules to document with autodoc) are in another directory,
# add these directories to sys.path here. If the directory is relative to the
# documentation root, use os.path.abspath to make it absolute, like shown here.

import os
import sys


sys.path.insert(0, os.path.abspath('.'))

def debug():
    def header(h: str):
        s = f"---- {h} "
        s += "-" * (80 - len(s))
        print(s)

    print("=" * 80)
    header("Current Directory")
    print(os.getcwd())
    header("Python Path")
    for p in sys.path:
        print(p)
    header("Python Executable")
    print(sys.executable)
    header("Prefix")
    print(f"{sys.prefix}")
    print(f"{sys.base_prefix}")
    print("=" * 80)

debug()

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'Pretty 🌸 Pretty'
copyright = '2024 Robert Grimm'
author = 'Robert Grimm'

# Source order is nicer than alphabetizing fields and methods.
autodoc_member_order = 'bysource'

# Known types are linked, so the fully qualified name adds mostly noise.
python_use_unqualified_type_names = True

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = [
    "sphinx.ext.napoleon",
    "sphinx.ext.autodoc",
    #"sphinx.ext.intersphinx",
    #"sphinx.ext.extlinks",
    "sphinx.ext.viewcode",
    "sphinx_copybutton",
    "sphinx_design",
    "sphinx_rtd_theme",
]

napoleon_include_init_with_doc = True
templates_path = ['_templates']
exclude_patterns = ['_build', 'Thumbs.db', '.DS_Store']
copybutton_exclude = '.linenos, .gp, .go'

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = 'sphinx_rtd_theme'
html_baseurl = 'python'
