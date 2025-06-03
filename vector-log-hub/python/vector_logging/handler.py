"""
VectorHandler implementation as specified in log-hub.md section 5.3.2

This handler sends Python logging records to Vector via TCP socket using
the standard JSON log format defined in the documentation.
"""

import json
import logging
import socket
import threading
import time
from datetime import datetime, timezone
from typing import Dict, Any, Optional


class VectorHandler(logging.Handler):
    """
    Logging handler that sends logs to Vector log aggregator via TCP socket.
    
    This implementation follows the specifications in log-hub.md section 5.3.2.
    """
    
    def __init__(self, host: str = 'localhost', port: int = 9000, 
                 service_name: str = 'python-service', timeout: float = 5.0):
        """
        Initialize VectorHandler
        
        Args:
            host: Vector TCP socket host (default: 'localhost')
            port: Vector TCP socket port (default: 9000)
            service_name: Name of the service for log identification
            timeout: Socket connection timeout in seconds
        """
        super().__init__()
        self.host = host
        self.port = port
        self.service_name = service_name
        self.timeout = timeout
        self._socket = None
        self._lock = threading.Lock()
        
    def _ensure_connection(self) -> bool:
        """Ensure TCP connection to Vector is established"""
        if self._socket is None:
            try:
                self._socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                self._socket.settimeout(self.timeout)
                self._socket.connect((self.host, self.port))
                return True
            except Exception as e:
                self._socket = None
                print(f"Failed to connect to Vector at {self.host}:{self.port}: {e}")
                return False
        return True
    
    def _send_to_vector(self, log_entry: Dict[str, Any]) -> bool:
        """Send log entry to Vector via TCP socket"""
        try:
            if not self._ensure_connection():
                return False
                
            log_json = json.dumps(log_entry, ensure_ascii=False)
            message = (log_json + '\n').encode('utf-8')
            self._socket.sendall(message)
            return True
            
        except Exception as e:
            print(f"Failed to send log to Vector: {e}")
            # Close connection on error to force reconnection
            self._close_connection()
            return False
    
    def _close_connection(self):
        """Close the TCP connection"""
        if self._socket:
            try:
                self._socket.close()
            except:
                pass
            finally:
                self._socket = None
    
    def emit(self, record: logging.LogRecord):
        """
        Emit a log record to Vector
        
        Converts logging records to the standard JSON log format specified in log-hub.md:
        {
          "timestamp": "2023-09-28T15:04:05Z",
          "level": "info",
          "message": "操作完成",
          "service": "服务名称",
          "context": {
            "operation_id": "abc123",
            "duration_ms": 42
          }
        }
        """
        try:
            with self._lock:
                # Format timestamp according to log-hub.md specification with microseconds
                timestamp = datetime.fromtimestamp(record.created, tz=timezone.utc)
                timestamp_str = timestamp.strftime('%Y-%m-%dT%H:%M:%S.%fZ')
                
                # Create base log entry
                log_entry = {
                    "timestamp": timestamp_str,
                    "level": record.levelname.lower(),
                    "message": self.format(record),
                    "service": self.service_name
                }
                
                # Extract context from record extras
                context = {}
                
                # Add standard context fields
                if hasattr(record, 'operation_id'):
                    context['operation_id'] = record.operation_id
                if hasattr(record, 'duration_ms'):
                    context['duration_ms'] = record.duration_ms
                if hasattr(record, 'user_id'):
                    context['user_id'] = record.user_id
                if hasattr(record, 'session_id'):
                    context['session_id'] = record.session_id
                
                # Add any other extra fields as context
                for key, value in record.__dict__.items():
                    if key not in ['name', 'msg', 'args', 'levelname', 'levelno', 
                                   'pathname', 'filename', 'module', 'lineno', 
                                   'funcName', 'created', 'msecs', 'relativeCreated',
                                   'thread', 'threadName', 'processName', 'process',
                                   'getMessage', 'exc_info', 'exc_text', 'stack_info']:
                        context[key] = value
                
                # Add context to log entry if not empty
                if context:
                    log_entry['context'] = context
                
                # Send to Vector
                self._send_to_vector(log_entry)
                
        except Exception as e:
            self.handleError(record)
    
    def close(self):
        """Close the handler and clean up resources"""
        with self._lock:
            self._close_connection()
        super().close()


def setup_logging(service_name: str = 'python-service', 
                  host: str = 'localhost', 
                  port: int = 9000,
                  level: int = logging.INFO) -> logging.Logger:
    """
    Setup logging with VectorHandler as specified in log-hub.md section 5.3.2
    
    Args:
        service_name: Name of the service for log identification
        host: Vector TCP socket host
        port: Vector TCP socket port
        level: Logging level
        
    Returns:
        Configured logger instance
        
    Example:
        logger = setup_logging('my-python-service')
        logger.info("Application started", extra={"version": "1.0"})
    """
    # Create VectorHandler
    handler = VectorHandler(host=host, port=port, service_name=service_name)
    
    # Set up formatter (though VectorHandler creates its own JSON format)
    formatter = logging.Formatter('%(message)s')
    handler.setFormatter(formatter)
    
    # Configure root logger
    logger = logging.getLogger()
    logger.setLevel(level)
    logger.addHandler(handler)
    
    return logger