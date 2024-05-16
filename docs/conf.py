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

sys.path.insert(0, os.path.abspath('..'))

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = 'prettypretty'
copyright = '© 2024 Robert Grimm'
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
    "sphinxcontrib.autoprogram",
    "sphinx_design",
    "sphinx_rtd_theme",
]

napoleon_include_init_with_doc = True
templates_path = ['_templates']
exclude_patterns = ['_build', 'Thumbs.db', '.DS_Store']

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

#html_theme = "sphinxawesome"
html_static_path = ['_static']
html_theme = 'sphinx_rtd_theme'
