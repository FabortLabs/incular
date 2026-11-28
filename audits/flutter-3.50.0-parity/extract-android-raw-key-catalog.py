#!/usr/bin/env python3
"""Translate the pinned Flutter Android key tables."""

from pathlib import Path
import runpy
import sys

sys.argv[1:1] = ["Android"]
runpy.run_path(str(Path(__file__).with_name("extract-numeric-raw-key-catalog.py")))
