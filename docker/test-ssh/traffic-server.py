import http.server
import random
import os

class TrafficHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        size = random.randint(1000, 100000)
        data = os.urandom(size)
        self.send_response(200)
        self.send_header("Content-Length", str(size))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *args):
        pass

import sys
port = int(sys.argv[1]) if len(sys.argv) > 1 else 80
print(f"Traffic server listening on port {port}", flush=True)
http.server.HTTPServer(("0.0.0.0", port), TrafficHandler).serve_forever()
