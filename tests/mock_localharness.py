#!/usr/bin/env python3
import sys
import struct
import socket
import asyncio
import websockets
import json

# Read InputConfig
# InputConfig length prefix (4 bytes)
len_bytes = sys.stdin.buffer.read(4)
if not len_bytes:
    sys.exit(1)
length = struct.unpack('<I', len_bytes)[0]
sys.stdin.buffer.read(length)

# Bind a TCP socket to a random available port to find one
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.bind(('127.0.0.1', 0))
port = s.getsockname()[1]
s.close()

# Write OutputConfig length and protobuf bytes
# OutputConfig has:
# field 1: port (int32)
# field 2: api_key (string)
# In proto encoding:
# port field: tag = (1 << 3) | 0 = 8.
# Varint for port:
port_bytes = b''
temp_port = port
while temp_port >= 0x80:
    port_bytes += bytes([ (temp_port & 0x7f) | 0x80 ])
    temp_port >>= 7
port_bytes += bytes([ temp_port ])

# api_key field: tag = (2 << 3) | 2 = 18. Length delimited.
# Value: "mock_api_key"
api_key_str = b"mock_api_key"
api_key_bytes = bytes([18, len(api_key_str)]) + api_key_str

proto_msg = bytes([8]) + port_bytes + api_key_bytes
length_prefix = struct.pack('<I', len(proto_msg))

sys.stdout.buffer.write(length_prefix)
sys.stdout.buffer.write(proto_msg)
sys.stdout.buffer.flush()

# WebSocket server
async def handler(websocket, *args):
    try:
        # First message is the HarnessConfig sent by client
        config_msg = await websocket.recv()
        
        # Now, send some mocked OutputEvents to the client
        # Let's send a StepUpdate with a user greeting
        step1 = {
            "stepUpdate": {
                "stepIndex": 1,
                "trajectoryId": "test_traj",
                "text": "Hello from mock harness!",
                "textDelta": "Hello from mock harness!",
                "state": "STATE_ACTIVE",
                "source": "SOURCE_MODEL",
                "target": "TARGET_USER"
            }
        }
        await websocket.send(json.dumps(step1))
        await asyncio.sleep(0.1)
        
        # Let's send a StepUpdate with complete response
        step2 = {
            "stepUpdate": {
                "stepIndex": 2,
                "trajectoryId": "test_traj",
                "text": "Hello from mock harness!How can I help you today?",
                "textDelta": "How can I help you today?",
                "state": "STATE_DONE",
                "source": "SOURCE_MODEL",
                "target": "TARGET_USER",
                "finish": {
                    "outputString": "\"done\""
                }
            }
        }
        await websocket.send(json.dumps(step2))
        
        # Keep connection open for a bit
        await asyncio.sleep(1.0)
    except Exception as e:
        sys.stderr.write(f"WebSocket error: {e}\n")

async def main():
    async with websockets.serve(handler, "127.0.0.1", port):
        await asyncio.Future()  # run forever

asyncio.run(main())
