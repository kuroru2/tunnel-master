import http.server
import random
import os
import sys
import threading

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

def serve(port):
    s = http.server.HTTPServer(("0.0.0.0", port), TrafficHandler)
    print(f"Traffic server listening on port {port}", flush=True)
    s.serve_forever()

# Start servers on all requested ports
ports = [int(p) for p in sys.argv[1:]] if len(sys.argv) > 1 else [9090]
threads = []
for port in ports:
    t = threading.Thread(target=serve, args=(port,), daemon=True)
    t.start()
    threads.append(t)

# Keep main thread alive
for t in threads:
    t.join()
