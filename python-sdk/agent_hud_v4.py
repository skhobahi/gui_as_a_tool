#!/usr/bin/env python3
"""
Agent HUD v5 - Python SDK
Self-contained desktop application integration with rich content support

This SDK automatically discovers and connects to the Agent HUD v5 desktop application
running on the local machine, with zero configuration required. Supports emitting
markdown, code, and images for rich content display.
"""

import json
import logging
import time
import uuid
import threading
from datetime import datetime
from typing import Optional, Dict, Any, List, Union
import websocket
import socket
from pathlib import Path

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class AgentHUDv4:
    """
    Agent HUD v5 SDK - Connects to the self-contained desktop application
    
    Features:
    - Auto-discovery of running Agent HUD v5 application
    - Zero configuration required
    - Human-in-the-loop interactions
    - Real-time bidirectional communication
    - Rich content support (markdown, code, images)
    """
    
    def __init__(
        self,
        agent_name: str = "Python Agent",
        metadata: Optional[Dict[str, Any]] = None,
        auto_connect: bool = True,
        discovery_timeout: int = 10
    ):
        """
        Initialize the Agent HUD v4 client.
        
        Args:
            agent_name: Display name for this agent
            metadata: Additional metadata about the agent
            auto_connect: Whether to connect immediately
            discovery_timeout: Timeout for discovering the HUD application
        """
        self.agent_name = agent_name
        self.metadata = metadata or {}
        self.connected = False
        self.agent_id = None
        self.ws = None
        self.server_port = None
        
        # Human-in-the-loop state
        self.pending_requests = {}
        self.request_responses = {}
        
        if auto_connect:
            if not self.discover_and_connect(timeout=discovery_timeout):
                raise ConnectionError("Could not find or connect to Agent HUD v4 application")
    
    def discover_and_connect(self, timeout: int = 10) -> bool:
        """
        Discover running Agent HUD v4 application and connect to it.
        
        Args:
            timeout: Timeout in seconds for discovery
            
        Returns:
            True if successfully connected, False otherwise
        """
        logger.info("Discovering Agent HUD v4 application...")
        
        # Try common ports where the HUD might be running
        ports_to_try = list(range(8080, 8200))
        
        for port in ports_to_try:
            if self._test_connection(port):
                logger.info(f"Found Agent HUD v4 on port {port}")
                self.server_port = port
                return self._connect_websocket()
                
        logger.error("Could not discover Agent HUD v4 application")
        logger.error("Make sure Agent HUD v4 desktop application is running")
        return False
    
    def _test_connection(self, port: int) -> bool:
        """Test if a WebSocket server is running on the given port."""
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(0.5)
            result = sock.connect_ex(('127.0.0.1', port))
            sock.close()
            return result == 0
        except Exception:
            return False
    
    def _connect_websocket(self) -> bool:
        """Connect to the WebSocket server."""
        try:
            ws_url = f"ws://127.0.0.1:{self.server_port}"
            logger.info(f"Connecting to Agent HUD v4 at {ws_url}")
            
            self.ws = websocket.WebSocketApp(
                ws_url,
                on_open=self._on_open,
                on_message=self._on_message,
                on_error=self._on_error,
                on_close=self._on_close
            )
            
            # Start WebSocket in a separate thread
            self.ws_thread = threading.Thread(target=self.ws.run_forever)
            self.ws_thread.daemon = True
            self.ws_thread.start()
            
            # Wait for connection
            max_wait = 5
            waited = 0
            while not self.connected and waited < max_wait:
                time.sleep(0.1)
                waited += 0.1
            
            return self.connected
            
        except Exception as e:
            logger.error(f"Failed to connect WebSocket: {e}")
            return False
    
    def _on_open(self, ws):
        """WebSocket connection opened."""
        logger.info("WebSocket connection established")
        
        # Register as agent
        registration_message = {
            "type": "register-agent",
            "name": self.agent_name,
            "metadata": self.metadata
        }
        
        self._send_message(registration_message)
    
    def _on_message(self, ws, message):
        """Handle incoming WebSocket message."""
        try:
            data = json.loads(message)
            message_type = data.get("type")
            
            if message_type == "registration-ack":
                self.connected = True
                self.agent_id = data.get("agentId")
                logger.info(f"Successfully registered as agent: {self.agent_id}")
                
            elif message_type == "human-input-response":
                self._handle_human_response(data)
                
            else:
                logger.debug(f"Received message: {message_type}")
                
        except Exception as e:
            logger.error(f"Error handling message: {e}")
    
    def _on_error(self, ws, error):
        """WebSocket error occurred."""
        logger.error(f"WebSocket error: {error}")
    
    def _on_close(self, ws, close_status_code, close_msg):
        """WebSocket connection closed."""
        logger.info("WebSocket connection closed")
        self.connected = False
        self.agent_id = None
    
    def _send_message(self, message: Dict[str, Any]) -> bool:
        """Send message through WebSocket."""
        if not self.ws:
            logger.warning("WebSocket not available - message not sent")
            return False
        
        # Allow registration messages to be sent even before fully connected
        if not self.connected and message.get("type") != "register-agent":
            logger.warning("Not connected - message not sent")
            return False
        
        try:
            self.ws.send(json.dumps(message))
            return True
        except Exception as e:
            logger.error(f"Failed to send message: {e}")
            return False
    
    def _handle_human_response(self, data: Dict[str, Any]):
        """Handle human response to a request."""
        request_id = data.get("requestId")
        if request_id in self.pending_requests:
            self.request_responses[request_id] = data
            # Notify waiting thread
            if 'event' in self.pending_requests[request_id]:
                self.pending_requests[request_id]['event'].set()
    
    def emit_markdown(
        self,
        content: str,
        title: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None
    ) -> str:
        """
        Emit markdown content to the HUD for rich display.
        
        Args:
            content: Markdown content to display
            title: Optional title for the content
            metadata: Additional metadata
            
        Returns:
            Message ID
        """
        message_id = str(uuid.uuid4())
        message = {
            "type": "markdown-content",
            "data": {
                "content": content,
                "title": title or "Markdown Content",
                "agent_id": self.agent_id,
                "agent_name": self.agent_name,
                "metadata": metadata,
                "timestamp": datetime.now().isoformat()
            }
        }
        
        self._send_message(message)
        return message_id
    
    def emit_code(
        self,
        code: str,
        language: str = "python",
        title: Optional[str] = None,
        description: Optional[str] = None
    ) -> str:
        """
        Emit code to the HUD with syntax highlighting.
        
        Args:
            code: Code content
            language: Programming language for syntax highlighting
            title: Optional title
            description: Optional description
            
        Returns:
            Message ID
        """
        message_id = str(uuid.uuid4())
        message = {
            "type": "code-content",
            "data": {
                "content": code,
                "language": language,
                "title": title or f"{language.title()} Code",
                "description": description,
                "agent_id": self.agent_id,
                "agent_name": self.agent_name,
                "timestamp": datetime.now().isoformat()
            }
        }
        
        self._send_message(message)
        return message_id
    
    def emit_image(
        self,
        image_url: str,
        title: Optional[str] = None,
        caption: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None
    ) -> str:
        """
        Emit an image to the HUD for display.
        
        Args:
            image_url: URL or base64 data URL of the image
            title: Optional title for the image
            caption: Optional caption text
            metadata: Additional metadata
            
        Returns:
            Message ID
        """
        message_id = str(uuid.uuid4())
        message = {
            "type": "image-content",
            "data": {
                "content": image_url,
                "title": title or "Image",
                "caption": caption,
                "agent_id": self.agent_id,
                "agent_name": self.agent_name,
                "metadata": metadata,
                "timestamp": datetime.now().isoformat()
            }
        }
        
        self._send_message(message)
        return message_id
    
    def emit_log(
        self,
        message: str,
        level: str = "info",
        source: Optional[str] = None,
        context: Optional[str] = None
    ) -> str:
        """
        Emit a log message to the HUD.
        
        Args:
            message: Log message
            level: Log level (debug, info, warning, error, success)
            source: Source of the log message
            context: Additional context
            
        Returns:
            Message ID
        """
        message_id = str(uuid.uuid4())
        msg = {
            "id": message_id,
            "type": "agent-message",
            "payload": {
                "type": "emit_log",
                "message": message,
                "level": level,
                "source": source or self.agent_name,
                "context": context
            },
            "timestamp": datetime.now().isoformat()
        }
        
        self._send_message(msg)
        return message_id
    
    def emit_notification(
        self,
        title: str,
        message: str,
        type: str = "info",
        priority: str = "medium"
    ) -> str:
        """
        Emit a notification to the HUD.
        
        Args:
            title: Notification title
            message: Notification message
            type: Notification type (info, success, warning, error)
            priority: Priority level (low, medium, high)
            
        Returns:
            Message ID
        """
        message_id = str(uuid.uuid4())
        msg = {
            "id": message_id,
            "type": "agent-message",
            "payload": {
                "type": "emit_notification",
                "title": title,
                "message": message,
                "type": type,
                "priority": priority
            },
            "timestamp": datetime.now().isoformat()
        }
        
        self._send_message(msg)
        return message_id
    
    def show_progress(
        self,
        current: int,
        total: int,
        message: Optional[str] = None,
        operation: Optional[str] = None
    ) -> str:
        """
        Show progress information in the HUD.
        
        Args:
            current: Current progress value
            total: Total progress value
            message: Progress message
            operation: Operation name
            
        Returns:
            Message ID
        """
        message_id = str(uuid.uuid4())
        msg = {
            "id": message_id,
            "type": "agent-message",
            "payload": {
                "type": "show_progress",
                "current": current,
                "total": total,
                "message": message,
                "operation": operation
            },
            "timestamp": datetime.now().isoformat()
        }
        
        self._send_message(msg)
        return message_id
    
    def request_human_input(
        self,
        message: str,
        input_type: str = "text",
        options: Optional[List[str]] = None,
        context: Optional[Dict[str, Any]] = None,
        timeout: int = 300
    ) -> Dict[str, Any]:
        """
        Request input from human operator.
        
        Args:
            message: The question/request message for the human
            input_type: Type of input requested ("text", "approval", "choice", etc.)
            options: List of available options for choice-type requests
            context: Additional context data for the human
            timeout: Timeout in seconds (default: 300 = 5 minutes)
            
        Returns:
            Dict containing the response from human or timeout info
        """
        if not self.connected:
            logger.error("Not connected to Agent HUD v4")
            return {"error": "Not connected", "response": None}
        
        # Create request
        request_id = f"req_{int(time.time())}_{uuid.uuid4().hex[:8]}"
        request_message = {
            "type": "human-input-request",
            "requestId": request_id,
            "message": message,
            "inputType": input_type,
            "options": options or [],
            "context": context or {},
            "timeout": timeout
        }
        
        # Create event for synchronization
        response_event = threading.Event()
        self.pending_requests[request_id] = {
            "event": response_event,
            "message": request_message,
            "timestamp": datetime.now().isoformat()
        }
        
        try:
            # Send request
            self._send_message(request_message)
            logger.info(f"Sent human input request: {message}")
            
            # Wait for response
            if response_event.wait(timeout=timeout + 5):  # Add 5s buffer
                response = self.request_responses.get(request_id, {})
                logger.info(f"Received human response: {response.get('response', 'No response')}")
                return response
            else:
                logger.warning(f"Human input request timed out after {timeout}s")
                return {
                    "requestId": request_id,
                    "response": None,
                    "timeout": True,
                    "message": "Request timed out"
                }
        
        finally:
            # Cleanup
            self.pending_requests.pop(request_id, None)
            self.request_responses.pop(request_id, None)
    
    def request_approval(
        self,
        action: str,
        context: Optional[Dict[str, Any]] = None,
        timeout: int = 300
    ) -> bool:
        """
        Request approval from human for a specific action.
        
        Args:
            action: Description of the action requiring approval
            context: Additional context about the action
            timeout: Timeout in seconds
            
        Returns:
            True if approved, False if rejected or timed out
        """
        response = self.request_human_input(
            message=f"Approval needed: {action}",
            input_type="approval",
            options=["Approve", "Reject"],
            context=context,
            timeout=timeout
        )
        
        if response.get("timeout") or response.get("error"):
            return False
        
        return response.get("response", "").lower() in ["approve", "approved", "yes", "y"]
    
    def request_choice(
        self,
        question: str,
        choices: List[str],
        context: Optional[Dict[str, Any]] = None,
        timeout: int = 300
    ) -> Optional[str]:
        """
        Request human to choose from a list of options.
        
        Args:
            question: The question to ask
            choices: List of available choices
            context: Additional context
            timeout: Timeout in seconds
            
        Returns:
            Selected choice or None if timed out/error
        """
        response = self.request_human_input(
            message=question,
            input_type="choice",
            options=choices,
            context=context,
            timeout=timeout
        )
        
        if response.get("timeout") or response.get("error"):
            return None
        
        return response.get("response")
    
    def request_context(
        self,
        query: str,
        timeout: int = 300
    ) -> Optional[str]:
        """
        Ask human for additional context or clarification.
        
        Args:
            query: What you need clarification about
            timeout: Timeout in seconds
            
        Returns:
            Human's response text or None if timed out/error
        """
        response = self.request_human_input(
            message=f"Need clarification: {query}",
            input_type="text",
            timeout=timeout
        )
        
        if response.get("timeout") or response.get("error"):
            return None
        
        return response.get("response")
    
    def confirm_action(
        self,
        action_description: str,
        details: Optional[Dict[str, Any]] = None,
        timeout: int = 300
    ) -> bool:
        """
        Ask human to confirm a critical action.
        
        Args:
            action_description: Description of the action
            details: Additional details about the action
            timeout: Timeout in seconds
            
        Returns:
            True if confirmed, False otherwise
        """
        context = {"action_details": details} if details else {}
        
        response = self.request_human_input(
            message=f"Confirm action: {action_description}",
            input_type="confirmation",
            options=["Confirm", "Cancel"],
            context=context,
            timeout=timeout
        )
        
        if response.get("timeout") or response.get("error"):
            return False
        
        return response.get("response", "").lower() in ["confirm", "confirmed", "yes", "ok"]
    
    def is_connected(self) -> bool:
        """Check if connected to Agent HUD v4."""
        return self.connected
    
    def disconnect(self):
        """Disconnect from Agent HUD v4."""
        if self.ws:
            self.ws.close()
        self.connected = False
        self.agent_id = None


# Convenience functions for quick usage
_default_hud = None

def connect_to_hud(
    agent_name: str = "Python Agent", 
    **kwargs
) -> AgentHUDv4:
    """Connect to Agent HUD v4 (convenience function)."""
    global _default_hud
    _default_hud = AgentHUDv4(agent_name, **kwargs)
    return _default_hud

def get_hud() -> Optional[AgentHUDv4]:
    """Get the default HUD instance."""
    return _default_hud

def emit_markdown(content: str, **kwargs) -> str:
    """Convenience function to emit markdown."""
    if _default_hud and _default_hud.is_connected():
        return _default_hud.emit_markdown(content, **kwargs)
    logger.warning("Not connected to Agent HUD v4")
    return ""

def emit_log(message: str, level: str = "info", **kwargs) -> str:
    """Convenience function to emit log."""
    if _default_hud and _default_hud.is_connected():
        return _default_hud.emit_log(message, level, **kwargs)
    logger.warning("Not connected to Agent HUD v4")
    return ""

def request_approval(action: str, **kwargs) -> bool:
    """Convenience function to request approval."""
    if _default_hud and _default_hud.is_connected():
        return _default_hud.request_approval(action, **kwargs)
    logger.warning("Not connected to Agent HUD v4")
    return False


if __name__ == "__main__":
    # Example usage
    print("🚀 Agent HUD v4 Python SDK - Example Usage")
    print("Make sure Agent HUD v4 desktop application is running!")
    
    try:
        # Connect to Agent HUD v4
        hud = AgentHUDv4("Demo Agent v4")
        
        print("✅ Connected to Agent HUD v4!")
        
        # Test basic messaging
        hud.emit_log("Agent started successfully", "info")
        hud.emit_markdown("# Agent HUD v4 Test\n\nThis is a **test** message from the Python SDK.")
        hud.emit_code("print('Hello from Agent HUD v4!')", "python", title="Test Code")
        
        # Test human-in-the-loop
        approval = hud.request_approval(
            "Test the approval system",
            context={"test": True, "timestamp": datetime.now().isoformat()}
        )
        
        if approval:
            hud.emit_notification("Approval Granted", "Test approved by human", "success")
        else:
            hud.emit_notification("Approval Denied", "Test rejected by human", "warning")
        
        # Test choice selection
        choice = hud.request_choice(
            "Which test should we run next?",
            ["Performance Test", "Security Test", "Integration Test", "Skip Tests"]
        )
        
        if choice:
            hud.emit_log(f"Running: {choice}", "info")
        
        time.sleep(2)
        hud.disconnect()
        print("✅ Demo completed successfully!")
        
    except ConnectionError as e:
        print(f"❌ {e}")
        print("Please start Agent HUD v4 desktop application first.")
    except Exception as e:
        print(f"❌ Unexpected error: {e}")