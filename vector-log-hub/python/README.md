# Vector Logging for Python

A Python logging handler that sends logs to Vector log aggregator via TCP socket, following the specifications in log-hub.md section 5.3.2.

## Features

- TCP socket connection to Vector on port 9000
- Standard JSON log format as specified in the documentation
- Thread-safe logging with automatic reconnection
- Support for structured logging with context data
- Compatible with Python's standard logging module

## Usage

```python
from vector_logging import setup_logging

# Initialize logging with Vector handler
logger = setup_logging('my-python-service', '127.0.0.1', 9000)

# Use standard logging
logger.info("Application started", extra={"version": "1.0"})
logger.error("Error occurred", extra={"operation_id": "abc123"})
```

## Installation

This package is part of the Vector Log Hub project and is designed for embedded systems logging.