#!/usr/bin/env python3
"""
Python tests for Vector logging integration

This test module validates the Python logging integration with Vector as specified 
in log-hub.md section 5.3.2. It demonstrates the VectorHandler and setup_logging 
functionality by sending logs to Vector via TCP socket.
"""

import logging
import time
import unittest
from vector_logging import VectorHandler, setup_logging


class TestVectorLogging(unittest.TestCase):
    """Test cases for Vector logging integration"""
    
    def setUp(self):
        """Set up test environment"""
        # Clear any existing handlers
        logging.getLogger().handlers.clear()
        
    def test_vector_handler_creation(self):
        """Test VectorHandler can be created with proper configuration"""
        handler = VectorHandler(
            host='127.0.0.1',
            port=9000,
            service_name='python-test-service'
        )
        
        self.assertEqual(handler.host, '127.0.0.1')
        self.assertEqual(handler.port, 9000)
        self.assertEqual(handler.service_name, 'python-test-service')
        print("✓ VectorHandler creation test passed")
    
    def test_setup_logging_function(self):
        """Test setup_logging function as specified in log-hub.md"""
        # Test the setup_logging function from the documentation
        logger = setup_logging('python-test-service', '127.0.0.1', 9000)
        
        self.assertIsInstance(logger, logging.Logger)
        self.assertTrue(any(isinstance(h, VectorHandler) for h in logger.handlers))
        print("✓ setup_logging function test passed")
    
    def test_basic_logging(self):
        """Test basic logging functionality"""
        # Initialize logging as shown in log-hub.md section 5.3.2
        logger = setup_logging('python-test-service', '127.0.0.1', 9000)
        
        # Test basic info logging
        logger.info("Test application started", extra={"version": "1.0"})
        logger.info("Python service initialized successfully")
        
        print("✓ Basic logging test completed")
    
    def test_logging_with_context(self):
        """Test logging with context data as specified in documentation"""
        logger = setup_logging('python-test-service', '127.0.0.1', 9000)
        
        # Test logging with context as shown in log-hub.md
        logger.info("User login", extra={
            "user_id": 12345,
            "session_id": "sess_abc123",
            "ip_address": "192.168.1.100",
            "operation_id": "login-op-456"
        })
        
        logger.info("Database operation completed", extra={
            "operation_id": "db-op-789",
            "duration_ms": 156,
            "table": "users",
            "rows_affected": 3
        })
        
        print("✓ Logging with context test completed")
    
    def test_different_log_levels(self):
        """Test different log levels"""
        logger = setup_logging('python-test-service', '127.0.0.1', 9000, logging.DEBUG)
        
        logger.debug("Debug message", extra={"component": "test"})
        logger.info("Info message", extra={"component": "test"})
        logger.warning("Warning message", extra={"component": "test"})
        logger.error("Error message", extra={"component": "test"})
        logger.critical("Critical message", extra={"component": "test"})
        
        print("✓ Different log levels test completed")
    
    def test_error_logging(self):
        """Test error logging with context"""
        logger = setup_logging('python-test-service', '127.0.0.1', 9000)
        
        try:
            # Simulate an error condition
            raise ValueError("Test error for logging")
        except Exception as e:
            logger.error("Application error occurred", extra={
                "error_type": type(e).__name__,
                "error_message": str(e),
                "operation_id": "error-test-123",
                "user_id": 999
            })
        
        print("✓ Error logging test completed")
    
    def test_structured_logging(self):
        """Test structured logging with various data types"""
        logger = setup_logging('python-test-service', '127.0.0.1', 9000)
        
        # Test with various data types in context
        logger.info("API request processed", extra={
            "method": "POST",
            "endpoint": "/api/users",
            "status_code": 201,
            "response_time_ms": 234,
            "success": True,
            "user_agent": "Python/3.9 requests/2.25.1",
            "operation_id": "api-req-789"
        })
        
        logger.info("File processing completed", extra={
            "file_name": "data.csv",
            "file_size_bytes": 1048576,
            "rows_processed": 10000,
            "processing_time_ms": 5432,
            "operation_id": "file-proc-456"
        })
        
        print("✓ Structured logging test completed")
    
    def test_rapid_logging(self):
        """Test Vector's buffering with rapid log generation"""
        logger = setup_logging('python-test-service', '127.0.0.1', 9000)
        
        # Test rapid logging to verify Vector's buffering capability
        for i in range(20):
            logger.info(f"Rapid log message #{i}", extra={
                "sequence": i,
                "batch_id": "rapid-test-python",
                "operation_id": f"rapid-op-{i}",
                "timestamp": int(time.time())
            })
            
            # Small delay to avoid overwhelming
            time.sleep(0.01)
        
        print("✓ Rapid logging test completed")
    
    def test_application_lifecycle_logging(self):
        """Test typical application lifecycle logging"""
        logger = setup_logging('python-test-service', '127.0.0.1', 9000)
        
        # Application startup
        logger.info("Application starting", extra={
            "version": "1.0.0",
            "environment": "test",
            "operation_id": "startup-001"
        })
        
        # Simulate some application work
        logger.info("Processing user request", extra={
            "user_id": 12345,
            "request_id": "req-abc123",
            "operation_id": "process-001"
        })
        
        # Simulate completion
        logger.info("Request processing completed", extra={
            "user_id": 12345,
            "request_id": "req-abc123",
            "operation_id": "process-001",
            "duration_ms": 150,
            "success": True
        })
        
        # Application shutdown
        logger.info("Application shutting down", extra={
            "operation_id": "shutdown-001",
            "uptime_seconds": 3600
        })
        
        print("✓ Application lifecycle logging test completed")


def run_integration_tests():
    """Run integration tests that demonstrate the examples from log-hub.md"""
    print("Running Python Vector logging integration tests...")
    print("=" * 60)
    
    # Example from log-hub.md section 5.3.2
    print("Testing setup_logging example from documentation:")
    logger = setup_logging('python-service', '127.0.0.1', 9000)
    logger.info("应用程序已启动", extra={"version": "1.0"})
    print("✓ Documentation example completed")
    print()
    
    # Run unit tests
    unittest.main(argv=[''], exit=False, verbosity=2)


if __name__ == "__main__":
    run_integration_tests()