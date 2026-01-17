import time
import argparse
import random
import string
import socket
import struct
import threading
from concurrent.futures import ThreadPoolExecutor

def generate_domain():
    return ''.join(random.choices(string.ascii_lowercase, k=8)) + ".com"

def build_query(domain):
    # Transaction ID
    tid = struct.pack(">H", random.randint(0, 65535))
    # Flags (Standard query, Recursion Desired)
    flags = b"\x01\x00"
    # Questions (1)
    questions = b"\x00\x01"
    # Answers, Authority, Additional
    others = b"\x00\x00\x00\x00\x00\x00"
    
    # Query section
    qname = b""
    for label in domain.split("."):
        qname += struct.pack("B", len(label)) + label.encode("utf-8")
    qname += b"\x00"
    
    # Type A (1), Class IN (1)
    qtype = b"\x00\x01"
    qclass = b"\x00\x01"
    
    return tid + flags + questions + others + qname + qtype + qclass

def send_query(domain, server, port):
    query = build_query(domain)
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(2.0)
    start = time.time()
    try:
        sock.sendto(query, (server, port))
        data, _ = sock.recvfrom(512)
        duration = (time.time() - start) * 1000
        return True, duration
    except Exception as e:
        return False, 0
    finally:
        sock.close()

def run_benchmark(total_requests, concurrency, server, port):
    print(f"Starting benchmark: {total_requests} requests to {server}:{port} with {concurrency} threads")
    
    domains = [generate_domain() for _ in range(total_requests)]
    
    success_count = 0
    fail_count = 0
    total_time = 0
    latencies = []

    start_time = time.time()
    
    with ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = {executor.submit(send_query, domain, server, port): domain for domain in domains}
        
        for i, future in enumerate(futures):
            success, duration = future.result()
            if success:
                success_count += 1
                total_time += duration
                latencies.append(duration)
            else:
                fail_count += 1
            
            if i % 1000 == 0 and i > 0:
                print(f"Processed {i} requests...")

    total_duration = time.time() - start_time
    qps = total_requests / total_duration
    avg_latency = total_time / success_count if success_count > 0 else 0
    
    print("\nBenchmark Results:")
    print(f"Total Requests: {total_requests}")
    print(f"Success: {success_count}")
    print(f"Failed: {fail_count}")
    print(f"Total Duration: {total_duration:.2f}s")
    print(f"QPS: {qps:.2f}")
    print(f"Avg Latency: {avg_latency:.2f}ms")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="DNS Benchmark")
    parser.add_argument("--count", type=int, default=5000, help="Number of queries")
    parser.add_argument("--threads", type=int, default=50, help="Concurrency")
    parser.add_argument("--server", type=str, default="127.0.0.1", help="DNS Server IP")
    parser.add_argument("--port", type=int, default=53, help="DNS Server Port")
    
    args = parser.parse_args()
    run_benchmark(args.count, args.threads, args.server, args.port)
