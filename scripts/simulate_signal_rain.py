#!/usr/bin/env python3
import time
import requests
import http.server
import threading
import json
import sys
from pathlib import Path

# CyberXiuXian Project Sentinel - Integrated Signal Rain Simulator (v2.0)
# This script acts as both a TRIGGER (modifying code) and a TRAP (receiving webhook).

TARGET_FILE = "packages/rust/crates/xiuxian-wendao/src/zhenfa_router/native/audit/audit_bridge.rs"
GATEWAY_URL = "http://127.0.0.1:9517"
MOCK_PORT = 9999

# Thread-safe storage for received signals
received_signals = []


class WebhookHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers["Content-Length"])
        post_data = self.rfile.read(content_length)
        try:
            signal = json.loads(post_data.decode("utf-8"))
            received_signals.append(signal)
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"status": "captured"}).encode())
        except Exception as e:
            print(f"\n❌ Error parsing webhook payload: {e}")
            self.send_response(400)
            self.end_headers()

    def log_message(self, format, *args):
        # Silence standard HTTP logs to keep output clean
        return


def start_mock_server():
    server_address = ("127.0.0.1", MOCK_PORT)
    httpd = http.server.HTTPServer(server_address, WebhookHandler)
    httpd.handle_request()  # Wait for exactly one POST request


def simulate_rain():
    print(f"🌧 Starting Integrated Signal Capture at port {MOCK_PORT}...")

    # 1. Start the trap in a background thread
    server_thread = threading.Thread(target=start_mock_server)
    server_thread.daemon = True
    server_thread.start()

    # 2. Verify target file existence
    target_path = Path(TARGET_FILE)
    if not target_path.exists():
        print(f"❌ Error: Target file {TARGET_FILE} not found!")
        sys.exit(1)

    original_content = target_path.read_text()

    try:
        # 3. Inject a semantic change
        print(f"🔨 Injecting change into {target_path.name}...")
        marker = int(time.time())
        target_path.write_text(
            original_content + f"\n// Project Sentinel Integration Test Marker: {marker}\n"
        )

        print("⌛ Waiting for Sentinel to detect change and Gateway to forward the signal...")

        # 4. Polling for the signal
        max_wait = 15  # Increased timeout for compile/debounce cycles
        start_time = time.time()
        while time.time() - start_time < max_wait:
            if received_signals:
                break
            # Print a small heartbeat to show we're alive
            print(".", end="", flush=True)
            time.sleep(1)

        print("\n")

        if received_signals:
            signal = received_signals[0]
            print("✅ SUCCESS! Programmatic verification complete.")
            print("--------------------------------------------------")
            print(f"📡 Signal Type: {signal.get('signal_type')}")
            print(f"🎯 Source:      {signal.get('source')}")
            print(f"📝 Summary:     {signal.get('summary')}")
            print(f"🛡 Confidence:  {signal.get('confidence')}")
            print(f"📂 Affected:    {', '.join(signal.get('affected_docs', []))}")
            print("--------------------------------------------------")
        else:
            print("❌ FAILED: Timeout waiting for signal.")
            print("💡 Check 'devenv up' logs for Sentinel/Gateway errors.")

    finally:
        # 5. Restore original content immediately
        target_path.write_text(original_content)
        print("✨ Environment cleaned up and restored.")


if __name__ == "__main__":
    simulate_rain()
