"""
Vector Logging Handler for Python

This module provides a logging handler that sends logs to Vector log aggregator
via TCP socket, following the specifications in log-hub.md section 5.3.2.
"""

from .handler import VectorHandler, setup_logging

__version__ = "0.1.0"
__all__ = ["VectorHandler", "setup_logging"]