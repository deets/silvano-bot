#!/usr/bin/env python3
"""
Silvano Bot Pure Python Web Server

- Opens on port 8080 (configurable via CLI).
- Serves the project's index.html.
- Under /move?left=<float>&right=<float>, converts floats in [-1.0, 1.0] to
  integers mapping -1.0 -> 0 and 1.0 -> 255.
- Sends these two bytes to a socket connecting to 192.168.4.1:80.
"""

import argparse
import json
import logging
import os
from pathlib import Path
import select
import socket
import sys
import threading
from http.server import HTTPServer, BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="[%(asctime)s] %(levelname)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("silvano_server")


def float_to_byte(val: float) -> int:
    """
    Map a float in [-1.0, 1.0] to an integer in [0, 255].
    -1.0 maps to 0, 1.0 maps to 255, and 0.0 maps to 128.
    Values outside [-1.0, 1.0] are clamped.
    """
    clamped = max(-1.0, min(1.0, float(-val)))
    scaled = (clamped + 1.0) / 2.0 * 255.0
    return max(0, min(255, int(round(scaled)))) >> 1


def find_index_html(custom_path: str | None = None) -> Path:
    """Locate index.html in the project tree."""
    if custom_path:
        p = Path(custom_path).resolve()
        if p.is_file():
            return p
        raise FileNotFoundError(f"Specified HTML file not found: {custom_path}")

    base_dir = Path(__file__).resolve().parent
    candidates = [
        base_dir / "silvano-bot-firmware" / "assets" / "index.html",
        base_dir / "assets" / "index.html",
        base_dir / "index.html",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate

    matches = list(base_dir.rglob("index.html"))
    if matches:
        return matches[0]

    raise FileNotFoundError("Could not locate index.html in project directory.")


class RobotSocketClient:
    """
    Manages the socket connection to the robot (target_ip:target_port).
    Maintains a persistent TCP connection when supported, and reconnects
    cleanly if disconnected or closed by the remote host.
    """

    def __init__(self, host: str = "192.168.4.1", port: int = 80, timeout: float = 1.0):
        self.host = host
        self.port = port
        self.timeout = timeout
        self._sock: socket.socket | None = None
        self._lock = threading.Lock()

    def _close(self):
        if self._sock is not None:
            try:
                self._sock.close()
            except Exception:
                pass
            self._sock = None

    def _is_closed(self) -> bool:
        if self._sock is None:
            return True
        try:
            # Check if socket has pending data or closed by remote (EOF)
            r, _, _ = select.select([self._sock], [], [], 0)
            if r:
                data = self._sock.recv(1, socket.MSG_PEEK)
                if not data:
                    return True
            return False
        except Exception:
            return True

    def send(self, data: bytes):
        """
        Send raw bytes to the robot socket.
        Automatically reconnects if the connection was closed.
        """
        with self._lock:
            if self._is_closed():
                self._close()

            # Attempt to send; retry once with a fresh connection if stale
            for attempt in range(2):
                if self._sock is None:
                    try:
                        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                        s.settimeout(self.timeout)
                        s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
                        s.connect((self.host, self.port))
                        self._sock = s
                    except Exception as e:
                        self._close()
                        raise e

                try:
                    self._sock.sendall(data)
                    return
                except Exception as e:
                    self._close()
                    if attempt == 1:
                        raise e

    def close(self):
        with self._lock:
            self._close()


class SilvanoRequestHandler(BaseHTTPRequestHandler):
    html_path: Path
    robot_client: RobotSocketClient

    def log_message(self, format, *args):
        logger.info("%s - %s", self.address_string(), format % args)

    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path

        if path in ("/", "/index.html"):
            self.handle_index()
        elif path == "/move":
            self.handle_move(parsed.query)
        elif path == "/favicon.ico":
            self.send_response(204)
            self.end_headers()
        else:
            self.send_error(404, "Not Found")

    def handle_index(self):
        try:
            with open(self.html_path, "rb") as f:
                content = f.read()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(content)))
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            self.wfile.write(content)
        except Exception as e:
            logger.error("Failed to read index.html: %s", e)
            self.send_error(500, f"Error reading index.html: {e}")

    def handle_move(self, query_str: str):
        params = parse_qs(query_str)
        try:
            left_val = float(params.get("left", ["0.0"])[0])
            right_val = float(params.get("right", ["0.0"])[0])
        except (ValueError, TypeError) as e:
            self.send_json_response(400, {
                "status": "error",
                "message": f"Invalid float parameters: {e}"
            })
            return

        left_byte = float_to_byte(left_val)
        right_byte = float_to_byte(right_val)

        payload = bytes([left_byte | 0x80, right_byte])
        try:
            self.robot_client.send(payload)
            logger.debug("Sent to robot: left=%d (0x%02x), right=%d (0x%02x)",
                         left_byte, left_byte, right_byte, right_byte)
            self.send_json_response(200, {
                "status": "ok",
                "left": left_byte,
                "right": right_byte,
            })
        except Exception as e:
            logger.warning("Failed to send movement to robot (%s:%s): %s",
                           self.robot_client.host, self.robot_client.port, e)
            self.send_json_response(502, {
                "status": "error",
                "message": f"Failed to send to robot at {self.robot_client.host}:{self.robot_client.port}: {e}",
                "left": left_byte,
                "right": right_byte,
            })

    def send_json_response(self, status_code: int, data: dict):
        body = json.dumps(data).encode("utf-8")
        self.send_response(status_code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
        self.end_headers()
        self.wfile.write(body)


def make_handler(html_path: Path, robot_client: RobotSocketClient):
    class Handler(SilvanoRequestHandler):
        pass

    Handler.html_path = html_path
    Handler.robot_client = robot_client
    return Handler


def run_server(
    bind: str = "0.0.0.0",
    port: int = 8080,
    target_ip: str = "192.168.4.1",
    target_port: int = 80,
    html_path: str | None = None,
):
    resolved_html = find_index_html(html_path)
    logger.info("Found index.html: %s", resolved_html)
    logger.info("Target robot socket: %s:%d", target_ip, target_port)

    robot_client = RobotSocketClient(host=target_ip, port=target_port)
    handler_class = make_handler(resolved_html, robot_client)

    server = ThreadingHTTPServer((bind, port), handler_class)
    server.daemon_threads = True

    logger.info("Starting webserver on http://%s:%d (Press Ctrl+C to stop)", bind, port)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        logger.info("Shutting down webserver...")
    finally:
        server.server_close()
        robot_client.close()
        logger.info("Webserver stopped.")


def main():
    parser = argparse.ArgumentParser(description="Silvano Bot Pure Python Web Server")
    parser.add_argument("--bind", "-b", default="0.0.0.0", help="Address to bind webserver (default: 0.0.0.0)")
    parser.add_argument("--port", "-p", type=int, default=8080, help="Port to listen on (default: 8080)")
    parser.add_argument("--target-ip", default="192.168.4.1", help="Target robot IP address (default: 192.168.4.1)")
    parser.add_argument("--target-port", type=int, default=80, help="Target robot port (default: 80)")
    parser.add_argument("--html", default=None, help="Path to index.html (default: auto-detect)")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose debug logging")

    args = parser.parse_args()

    if args.verbose:
        logger.setLevel(logging.DEBUG)

    run_server(
        bind=args.bind,
        port=args.port,
        target_ip=args.target_ip,
        target_port=args.target_port,
        html_path=args.html,
    )


if __name__ == "__main__":
    main()
